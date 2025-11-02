//! IPPAN Time Synchronization Service
//!
//! Provides a lightweight libp2p request/response protocol that
//! periodically exchanges timestamp samples between peers. Incoming
//! samples are fed into [`crate::ippan_time::ingest_sample`] to keep
//! deterministic IPPAN Time aligned with the peer median.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use anyhow::Context;
use async_trait::async_trait;
use ed25519_dalek::SigningKey;
use libp2p::core::upgrade;
use libp2p::futures::io::{AsyncReadExt, AsyncWriteExt};
use libp2p::futures::StreamExt;
use libp2p::identity;
use libp2p::noise;
use libp2p::request_response::{
    self, Config as RequestResponseConfig, Event as RequestResponseEvent,
    Message as RequestResponseMessage, ProtocolSupport,
};
use libp2p::swarm::{Executor, SwarmEvent};
use libp2p::tcp;
use libp2p::yamux;
use libp2p::{Multiaddr, PeerId, Swarm, Transport};
use log::{info, warn};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use tokio::time::interval;

use crate::hashtimer::{sign_hashtimer, verify_hashtimer};
use crate::ippan_time::{ingest_sample, now_us};

const PROTOCOL_NAME: &str = "/ippan/time/1.0.0";
const SYNC_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, Default)]
struct TokioExecutor;

impl Executor for TokioExecutor {
    fn exec(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>) {
        tokio::spawn(future);
    }
}

/// Request payload for time synchronization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimeRequest;

/// Response payload carrying the remote peer's perception of IPPAN time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimeResponse {
    pub time_us: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TimeProtocol;

impl AsRef<str> for TimeProtocol {
    fn as_ref(&self) -> &str {
        PROTOCOL_NAME
    }
}

/// Codec used for serializing [`TimeRequest`] and [`TimeResponse`] messages.
#[derive(Clone, Default)]
struct TimeCodec;

#[async_trait]
impl request_response::Codec for TimeCodec {
    type Protocol = TimeProtocol;
    type Request = TimeRequest;
    type Response = TimeResponse;

    async fn read_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> std::io::Result<Self::Request>
    where
        T: libp2p::futures::AsyncRead + Unpin + Send,
    {
        let mut buf = Vec::new();
        io.read_to_end(&mut buf).await?;
        serde_json::from_slice(&buf)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))
    }

    async fn read_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> std::io::Result<Self::Response>
    where
        T: libp2p::futures::AsyncRead + Unpin + Send,
    {
        let mut buf = Vec::new();
        io.read_to_end(&mut buf).await?;
        serde_json::from_slice(&buf)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))
    }

    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        request: Self::Request,
    ) -> std::io::Result<()>
    where
        T: libp2p::futures::AsyncWrite + Unpin + Send,
    {
        let payload = serde_json::to_vec(&request)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        io.write_all(&payload).await?;
        io.close().await
    }

    async fn write_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        response: Self::Response,
    ) -> std::io::Result<()>
    where
        T: libp2p::futures::AsyncWrite + Unpin + Send,
    {
        let payload = serde_json::to_vec(&response)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        io.write_all(&payload).await?;
        io.close().await
    }
}

type TimeSyncBehaviour = request_response::Behaviour<TimeCodec>;

fn build_time_sync_behaviour() -> TimeSyncBehaviour {
    let config = RequestResponseConfig::default().with_request_timeout(SYNC_INTERVAL * 3);
    let protocols = std::iter::once((TimeProtocol, ProtocolSupport::Full));
    request_response::Behaviour::new(protocols, config)
}

/// Background service that maintains a libp2p swarm dedicated to time exchange.
pub struct TimeSyncService;

impl TimeSyncService {
    /// Start the synchronization service bound to the provided `listen_addr`.
    ///
    /// The service responds to inbound time requests and periodically
    /// broadcasts its own requests to connected peers.
    pub async fn start(listen_addr: &str) -> anyhow::Result<()> {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        info!("Starting IPPAN time sync as {local_peer_id}");

        let mut key_rng = OsRng;
        let signing_key = SigningKey::generate(&mut key_rng);

        let noise = noise::Config::new(&local_key).context("failed to initialize noise config")?;

        let transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
            .upgrade(upgrade::Version::V1)
            .authenticate(noise)
            .multiplex(yamux::Config::default())
            .boxed();

        let behaviour = build_time_sync_behaviour();

        let swarm_config = libp2p::swarm::Config::with_executor(TokioExecutor)
            .with_idle_connection_timeout(SYNC_INTERVAL * 3);
        let mut swarm = Swarm::new(transport, behaviour, local_peer_id, swarm_config);

        let addr: Multiaddr = listen_addr.parse().context("invalid listen multiaddr")?;
        Swarm::listen_on(&mut swarm, addr.clone())
            .context("failed to start IPPAN time sync listener")?;

        let mut ticker = interval(SYNC_INTERVAL);

        loop {
            tokio::select! {
                event = swarm.select_next_some() => {
                    match event {
                        SwarmEvent::NewListenAddr { address, .. } => {
                            info!("Time sync listening on {address}");
                        }
                        SwarmEvent::Behaviour(RequestResponseEvent::Message { peer, message }) => {
                            match message {
                                RequestResponseMessage::Request { channel, .. } => {
                                    let response = TimeResponse { time_us: now_us() };
                                    if swarm.behaviour_mut().send_response(channel, response).is_err() {
                                        warn!("failed to send time response to {peer}");
                                    }
                                }
                                RequestResponseMessage::Response { response, .. } => {
                                    ingest_sample(response.time_us);
                                }
                            }
                        }
                        SwarmEvent::Behaviour(RequestResponseEvent::OutboundFailure { peer, error, .. }) => {
                            warn!("time request to {peer} failed: {error:?}");
                        }
                        SwarmEvent::Behaviour(RequestResponseEvent::InboundFailure { peer, error, .. }) => {
                            warn!("time response from {peer} failed: {error:?}");
                        }
                        _ => {}
                    }
                }
                _ = ticker.tick() => {
                    let hashtimer = sign_hashtimer(&signing_key);
                    if verify_hashtimer(&hashtimer) {
                        info!("⏱  HashTimer emitted: {} ({} µs)", hashtimer.id_hex(), hashtimer.timestamp_us);
                    } else {
                        warn!("generated HashTimer failed verification");
                    }
                    let peers: Vec<PeerId> = swarm.connected_peers().cloned().collect();
                    for peer in peers {
                        swarm.behaviour_mut().send_request(&peer, TimeRequest);
                    }
                }
            }
        }
    }
}

/// Spawn the synchronization service as a detached Tokio task.
pub async fn start_time_sync(listen_addr: &str) {
    let addr = listen_addr.to_owned();
    tokio::spawn(async move {
        if let Err(err) = TimeSyncService::start(&addr).await {
            warn!("IPPAN time sync service stopped: {err:?}");
        }
    });
}

#[cfg(all(test, feature = "enable-tests"))]
mod tests {
    use super::*;

    #[test]
    fn codec_roundtrips() {
        let request = serde_json::to_vec(&TimeRequest).unwrap();
        assert_eq!(
            serde_json::from_slice::<TimeRequest>(&request).unwrap(),
            TimeRequest
        );

        let response = TimeResponse { time_us: 42 };
        let payload = serde_json::to_vec(&response).unwrap();
        assert_eq!(
            serde_json::from_slice::<TimeResponse>(&payload)
                .unwrap()
                .time_us,
            42
        );
    }
}
