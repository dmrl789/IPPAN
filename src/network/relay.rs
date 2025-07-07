//! Relay module for NAT traversal
//! 
//! Handles peer relay functionality for nodes behind NAT.

use crate::{error::IppanError, Result};
use libp2p::{
    core::upgrade,
    relay::v2::{
        client::{self, Client},
        server::{self, Server},
        Event as RelayEvent, Protocol, Reservation, ReservationId,
    },
    swarm::{NetworkBehaviour, NotifyHandler, OneShotHandler, ToSwarm},
    PeerId, StreamProtocol,
};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Relay behaviour for NAT traversal
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "RelayEvent")]
pub struct RelayBehaviour {
    /// Relay client for requesting relay connections
    pub client: Client,
    /// Relay server for providing relay services
    pub server: Server,
}

/// Relay events
#[derive(Debug)]
pub enum RelayEvent {
    /// Client event
    Client(client::Event),
    /// Server event
    Server(server::Event),
}

impl RelayBehaviour {
    /// Create a new relay behaviour
    pub fn new() -> Self {
        Self {
            client: Client::new(PeerId::random()),
            server: Server::new(PeerId::random()),
        }
    }
    
    /// Request a relay reservation
    pub fn request_reservation(&mut self, relay_peer: PeerId) -> Result<()> {
        self.client.request_reservation(relay_peer);
        Ok(())
    }
    
    /// Dial a peer through a relay
    pub fn dial_peer_via_relay(
        &mut self,
        relay_peer: PeerId,
        remote_peer: PeerId,
        protocol: StreamProtocol,
    ) -> Result<()> {
        self.client.dial_peer_via_relay(relay_peer, remote_peer, protocol);
        Ok(())
    }
    
    /// Accept a relay reservation
    pub fn accept_reservation(&mut self, reservation: Reservation) -> Result<()> {
        self.server.accept_reservation(reservation);
        Ok(())
    }
    
    /// Reject a relay reservation
    pub fn reject_reservation(&mut self, reservation: Reservation) -> Result<()> {
        self.server.reject_reservation(reservation);
        Ok(())
    }
}

/// Relay manager
pub struct RelayManager {
    /// Active relay connections
    active_relays: HashMap<PeerId, RelayConnection>,
    /// Pending relay requests
    pending_requests: HashMap<ReservationId, RelayRequest>,
    /// Relay configuration
    config: RelayConfig,
}

/// Relay connection information
#[derive(Debug, Clone)]
pub struct RelayConnection {
    /// Relay peer ID
    pub relay_peer: PeerId,
    /// Remote peer ID
    pub remote_peer: PeerId,
    /// Connection direction
    pub direction: RelayDirection,
    /// Connection timestamp
    pub established_at: chrono::DateTime<chrono::Utc>,
    /// Protocol being relayed
    pub protocol: StreamProtocol,
}

/// Relay direction
#[derive(Debug, Clone)]
pub enum RelayDirection {
    /// Outbound relay connection
    Outbound,
    /// Inbound relay connection
    Inbound,
}

/// Relay request
#[derive(Debug, Clone)]
pub struct RelayRequest {
    /// Requesting peer
    pub peer: PeerId,
    /// Request timestamp
    pub requested_at: chrono::DateTime<chrono::Utc>,
    /// Request protocol
    pub protocol: Option<StreamProtocol>,
}

/// Relay configuration
#[derive(Debug, Clone)]
pub struct RelayConfig {
    /// Maximum relay connections
    pub max_relay_connections: usize,
    /// Relay reservation timeout
    pub reservation_timeout: std::time::Duration,
    /// Enable relay server
    pub enable_server: bool,
    /// Enable relay client
    pub enable_client: bool,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            max_relay_connections: 100,
            reservation_timeout: std::time::Duration::from_secs(300), // 5 minutes
            enable_server: true,
            enable_client: true,
        }
    }
}

impl RelayManager {
    /// Create a new relay manager
    pub fn new(config: RelayConfig) -> Self {
        Self {
            active_relays: HashMap::new(),
            pending_requests: HashMap::new(),
            config,
        }
    }
    
    /// Handle relay event
    pub fn handle_event(&mut self, event: RelayEvent) -> Result<()> {
        match event {
            RelayEvent::Client(event) => {
                self.handle_client_event(event)?;
            }
            RelayEvent::Server(event) => {
                self.handle_server_event(event)?;
            }
        }
        Ok(())
    }
    
    /// Handle client events
    fn handle_client_event(&mut self, event: client::Event) -> Result<()> {
        match event {
            client::Event::ReservationReqAccepted { relay_peer } => {
                info!("Relay reservation accepted by {}", relay_peer);
            }
            client::Event::ReservationReqFailed { relay_peer, error } => {
                warn!("Relay reservation failed with {}: {}", relay_peer, error);
            }
            client::Event::OutboundConnectEstablished { relay_peer, remote_peer } => {
                info!("Relay connection established: {} -> {} via {}", remote_peer, relay_peer, relay_peer);
                
                let connection = RelayConnection {
                    relay_peer,
                    remote_peer,
                    direction: RelayDirection::Outbound,
                    established_at: chrono::Utc::now(),
                    protocol: StreamProtocol::new("ippan/1.0"),
                };
                
                self.active_relays.insert(remote_peer, connection);
            }
            client::Event::OutboundConnectFailed { relay_peer, remote_peer, error } => {
                warn!("Relay connection failed: {} -> {} via {}: {}", remote_peer, relay_peer, relay_peer, error);
            }
        }
        Ok(())
    }
    
    /// Handle server events
    fn handle_server_event(&mut self, event: server::Event) -> Result<()> {
        match event {
            server::Event::ReservationReqReceived { relay_peer, reservation_id } => {
                debug!("Relay reservation request from {}: {}", relay_peer, reservation_id);
                
                let request = RelayRequest {
                    peer: relay_peer,
                    requested_at: chrono::Utc::now(),
                    protocol: None,
                };
                
                self.pending_requests.insert(reservation_id, request);
            }
            server::Event::InboundConnectEstablished { relay_peer, remote_peer } => {
                info!("Inbound relay connection: {} -> {} via {}", remote_peer, relay_peer, relay_peer);
                
                let connection = RelayConnection {
                    relay_peer,
                    remote_peer,
                    direction: RelayDirection::Inbound,
                    established_at: chrono::Utc::now(),
                    protocol: StreamProtocol::new("ippan/1.0"),
                };
                
                self.active_relays.insert(remote_peer, connection);
            }
            server::Event::InboundConnectFailed { relay_peer, remote_peer, error } => {
                warn!("Inbound relay connection failed: {} -> {} via {}: {}", remote_peer, relay_peer, relay_peer, error);
            }
        }
        Ok(())
    }
    
    /// Get active relay connections
    pub fn active_relays(&self) -> &HashMap<PeerId, RelayConnection> {
        &self.active_relays
    }
    
    /// Get pending relay requests
    pub fn pending_requests(&self) -> &HashMap<ReservationId, RelayRequest> {
        &self.pending_requests
    }
    
    /// Clean up expired requests
    pub fn cleanup_expired_requests(&mut self) {
        let now = chrono::Utc::now();
        let timeout = chrono::Duration::from_std(self.config.reservation_timeout).unwrap();
        
        self.pending_requests.retain(|_, request| {
            now.signed_duration_since(request.requested_at) < timeout
        });
    }
    
    /// Check if we can accept more relay connections
    pub fn can_accept_relay(&self) -> bool {
        self.active_relays.len() < self.config.max_relay_connections
    }
    
    /// Get relay statistics
    pub fn get_stats(&self) -> RelayStats {
        RelayStats {
            active_relays: self.active_relays.len(),
            pending_requests: self.pending_requests.len(),
            max_relay_connections: self.config.max_relay_connections,
        }
    }
}

/// Relay statistics
#[derive(Debug, Clone)]
pub struct RelayStats {
    /// Number of active relay connections
    pub active_relays: usize,
    /// Number of pending requests
    pub pending_requests: usize,
    /// Maximum relay connections
    pub max_relay_connections: usize,
}

impl Default for RelayBehaviour {
    fn default() -> Self {
        Self::new()
    }
}
