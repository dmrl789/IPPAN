use std::time::Duration;

use clap::Parser;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
#[command(
    name = "ippan-check-nodes",
    about = "Validate IPPAN node health endpoints and peer connectivity"
)]
struct Cli {
    /// Base URLs for the node RPC APIs (e.g. http://127.0.0.1:8080)
    #[arg(long = "api", value_name = "URL", num_args = 1.., value_delimiter = ',')]
    api_bases: Vec<String>,

    /// Path to the health endpoint (defaults to /health)
    #[arg(long, value_name = "PATH", default_value = "/health")]
    health_path: String,

    /// Path to the status endpoint (defaults to /status)
    #[arg(long, value_name = "PATH", default_value = "/status")]
    status_path: String,

    /// Path to the peers endpoint (defaults to /peers)
    #[arg(long, value_name = "PATH", default_value = "/peers")]
    peers_path: String,

    /// Minimum peer count required for the node to be considered connected.
    /// When omitted the checker expects each node to see every other node as a peer.
    #[arg(long)]
    require_peers: Option<usize>,

    /// Timeout (in seconds) for each HTTP request
    #[arg(long, default_value_t = 10)]
    timeout_seconds: u64,

    /// Emit JSON instead of a human readable table
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Deserialize, Clone, Default)]
struct HealthPayload {
    status: Option<String>,
    node_id: Option<String>,
    version: Option<String>,
    peer_count: Option<usize>,
}

#[derive(Debug, Deserialize, Clone, Default)]
struct StatusPayload {
    version: Option<String>,
    #[serde(flatten)]
    #[allow(dead_code)]
    extra: serde_json::Value,
}

#[derive(Debug, Deserialize, Clone, Default)]
struct PeersPayload {
    peers: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize, Default)]
struct NodeReport {
    api_base: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    node_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    health_status_code: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    peers_status_code: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status_status_code: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    health_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    peer_count_reported: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    peer_count_listed: Option<usize>,
    healthy: bool,
    connected: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    messages: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<String>,
}

impl NodeReport {
    fn flag_error(&mut self, message: impl Into<String>) {
        self.errors.push(message.into());
    }

    fn add_message(&mut self, message: impl Into<String>) {
        self.messages.push(message.into());
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Cli {
        api_bases,
        health_path,
        status_path,
        peers_path,
        require_peers,
        timeout_seconds,
        json,
    } = Cli::parse();

    let require_peers = require_peers.unwrap_or_else(|| api_bases.len().saturating_sub(1));

    let client = Client::builder()
        .timeout(Duration::from_secs(timeout_seconds))
        .build()?;

    let mut reports = Vec::with_capacity(api_bases.len());

    for base in &api_bases {
        let report = check_node(
            &client,
            base,
            &health_path,
            &status_path,
            &peers_path,
            require_peers,
        )
        .await;
        reports.push(report);
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&reports)?);
    } else {
        print_table(&reports);
    }

    let exit_code = if reports.iter().any(|r| !r.healthy) {
        1
    } else if reports.iter().any(|r| !r.connected) {
        2
    } else {
        0
    };

    if exit_code == 0 {
        Ok(())
    } else {
        std::process::exit(exit_code);
    }
}

async fn check_node(
    client: &Client,
    base: &str,
    health_path: &str,
    status_path: &str,
    peers_path: &str,
    require_peers: usize,
) -> NodeReport {
    let mut report = NodeReport {
        api_base: base.to_string(),
        ..Default::default()
    };

    let health_url = join_url(base, health_path);
    let status_url = join_url(base, status_path);
    let peers_url = join_url(base, peers_path);

    let (health_code, health_payload, health_error) =
        fetch_json::<HealthPayload>(client, &health_url).await;
    report.health_status_code = health_code;
    if let Some(err) = health_error {
        report.flag_error(format!("health: {err}"));
    }
    if let Some(payload) = health_payload {
        report.health_status = payload.status.clone();
        report.node_id = payload.node_id.clone();
        if report.version.is_none() {
            report.version = payload.version.clone();
        }
        report.peer_count_reported = payload.peer_count;
    }

    let (status_code, status_payload, status_error) =
        fetch_json::<StatusPayload>(client, &status_url).await;
    report.status_status_code = status_code;
    if let Some(err) = status_error {
        report.flag_error(format!("status: {err}"));
    }
    if let Some(payload) = status_payload {
        if report.version.is_none() {
            report.version = payload.version;
        }
    }

    let (peers_code, peers_payload, peers_error) =
        fetch_json::<PeersPayload>(client, &peers_url).await;
    report.peers_status_code = peers_code;
    if let Some(err) = peers_error {
        report.flag_error(format!("peers: {err}"));
    }
    if let Some(payload) = peers_payload {
        let count = payload.peers.map(|p| p.len());
        report.peer_count_listed = count;
    }

    evaluate_connectivity(&mut report, require_peers);

    report
}

fn evaluate_connectivity(report: &mut NodeReport, require_peers: usize) {
    let status_ok = report
        .health_status
        .as_deref()
        .map(|s| s.eq_ignore_ascii_case("healthy"))
        .unwrap_or(false);
    let health_ok = report.health_status_code == Some(200) && status_ok;
    report.healthy = health_ok;

    if !health_ok {
        if report.health_status_code != Some(200) {
            report.add_message("health endpoint did not return HTTP 200");
        }
        if !status_ok {
            report.add_message("health payload did not report status=healthy");
        }
    }

    let reported = report.peer_count_reported.unwrap_or(0);
    let listed = report.peer_count_listed.unwrap_or(0);
    let observed = reported.max(listed);

    let peers_ok = observed >= require_peers;
    report.connected = health_ok && peers_ok;

    if !peers_ok {
        report.add_message(format!(
            "peer count below required minimum ({require_peers}) — reported={reported}, listed={listed}"
        ));
    }

    if report.peers_status_code != Some(200) {
        report.add_message("peers endpoint not reachable (non-200 response)");
    }
}

async fn fetch_json<T>(client: &Client, url: &str) -> (Option<u16>, Option<T>, Option<String>)
where
    T: for<'de> Deserialize<'de>,
{
    match client.get(url).send().await {
        Ok(response) => {
            let status = response.status();
            let code = status.as_u16();
            match response.bytes().await {
                Ok(bytes) => {
                    if status.is_success() {
                        match serde_json::from_slice::<T>(&bytes) {
                            Ok(payload) => (Some(code), Some(payload), None),
                            Err(err) => (
                                Some(code),
                                None,
                                Some(format!(
                                    "failed to parse JSON ({} bytes): {}",
                                    bytes.len(),
                                    err
                                )),
                            ),
                        }
                    } else {
                        let snippet = String::from_utf8_lossy(&bytes);
                        (
                            Some(code),
                            None,
                            Some(format!("HTTP {} body: {}", code, truncate(&snippet))),
                        )
                    }
                }
                Err(err) => (
                    Some(code),
                    None,
                    Some(format!("failed to read body: {err}")),
                ),
            }
        }
        Err(err) => (None, None, Some(err.to_string())),
    }
}

fn join_url(base: &str, path: &str) -> String {
    let trimmed = base.trim_end_matches('/');
    if path.is_empty() {
        return trimmed.to_string();
    }
    if path.starts_with('/') {
        format!("{trimmed}{path}")
    } else {
        format!("{trimmed}/{path}")
    }
}

fn truncate(text: &str) -> String {
    const LIMIT: usize = 120;
    if text.len() <= LIMIT {
        text.to_string()
    } else {
        format!("{}…", &text[..LIMIT])
    }
}

fn print_table(reports: &[NodeReport]) {
    for report in reports {
        println!("Node: {}", report.api_base);
        println!(
            "  Health: {} (code: {:?}, status: {})",
            if report.healthy { "✅" } else { "❌" },
            report.health_status_code,
            report.health_status.as_deref().unwrap_or("<missing>")
        );
        println!(
            "  Peers: {} (reported: {:?}, listed: {:?})",
            if report.connected {
                "✅ connected"
            } else {
                "❌ disconnected"
            },
            report.peer_count_reported,
            report.peer_count_listed
        );
        if let Some(version) = &report.version {
            println!("  Version: {version}");
        }
        if let Some(node_id) = &report.node_id {
            println!("  Node ID: {node_id}");
        }
        for msg in &report.messages {
            println!("  • {msg}");
        }
        for err in &report.errors {
            println!("  ! {err}");
        }
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluate_connectivity_marks_connected_when_threshold_met() {
        let mut report = NodeReport {
            health_status_code: Some(200),
            health_status: Some("healthy".into()),
            peers_status_code: Some(200),
            peer_count_reported: Some(2),
            peer_count_listed: Some(1),
            ..Default::default()
        };
        evaluate_connectivity(&mut report, 1);
        assert!(report.healthy);
        assert!(report.connected);
        assert!(report.messages.is_empty());
    }

    #[test]
    fn evaluate_connectivity_marks_disconnected_on_low_peers() {
        let mut report = NodeReport {
            health_status_code: Some(200),
            health_status: Some("healthy".into()),
            peer_count_reported: Some(0),
            peer_count_listed: Some(0),
            ..Default::default()
        };
        evaluate_connectivity(&mut report, 1);
        assert!(report.healthy);
        assert!(!report.connected);
        assert!(report
            .messages
            .iter()
            .any(|m| m.contains("peer count below required minimum")));
    }

    #[test]
    fn evaluate_connectivity_marks_unhealthy_on_bad_status() {
        let mut report = NodeReport {
            health_status_code: Some(500),
            health_status: Some("unhealthy".into()),
            ..Default::default()
        };
        evaluate_connectivity(&mut report, 1);
        assert!(!report.healthy);
        assert!(!report.connected);
        assert!(report
            .messages
            .iter()
            .any(|m| m.contains("health endpoint did not return HTTP 200")));
    }

    #[test]
    fn join_url_handles_trailing_and_leading_slashes() {
        assert_eq!(
            join_url("http://localhost:8080/", "/health"),
            "http://localhost:8080/health"
        );
        assert_eq!(
            join_url("http://localhost:8080", "health"),
            "http://localhost:8080/health"
        );
        assert_eq!(
            join_url("http://localhost:8080/", "status"),
            "http://localhost:8080/status"
        );
    }

    #[test]
    fn truncate_limits_length() {
        assert_eq!(truncate("short"), "short");
        let long = "a".repeat(200);
        let truncated = truncate(&long);
        assert!(truncated.ends_with('…'));
        assert!(truncated.len() < long.len());
    }
}
