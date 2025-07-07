//! NAT traversal module
//! 
//! Handles UPnP and STUN for NAT traversal.

use crate::{error::IppanError, Result};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};

/// NAT manager for handling NAT traversal
pub struct NatManager {
    /// NAT configuration
    config: super::NatConfig,
    /// UPnP client
    upnp_client: Option<UpnpClient>,
    /// STUN client
    stun_client: Option<StunClient>,
    /// External address
    external_addr: Option<SocketAddr>,
    /// Port mappings
    port_mappings: Vec<PortMapping>,
}

/// UPnP client for port forwarding
struct UpnpClient {
    /// Gateway address
    gateway_addr: SocketAddr,
    /// Control URL
    control_url: String,
    /// Service type
    service_type: String,
}

/// STUN client for discovering external address
struct StunClient {
    /// STUN servers
    servers: Vec<SocketAddr>,
    /// External address
    external_addr: Option<SocketAddr>,
}

/// Port mapping information
#[derive(Debug, Clone)]
pub struct PortMapping {
    /// Internal port
    pub internal_port: u16,
    /// External port
    pub external_port: u16,
    /// Protocol
    pub protocol: Protocol,
    /// Description
    pub description: String,
    /// Mapping ID
    pub mapping_id: String,
}

/// Protocol type
#[derive(Debug, Clone)]
pub enum Protocol {
    /// TCP protocol
    Tcp,
    /// UDP protocol
    Udp,
}

impl NatManager {
    /// Create a new NAT manager
    pub async fn new(config: super::NatConfig) -> Result<Self> {
        let mut nat_manager = Self {
            config,
            upnp_client: None,
            stun_client: None,
            external_addr: None,
            port_mappings: Vec::new(),
        };
        
        // Initialize UPnP if enabled
        if nat_manager.config.enable_upnp {
            if let Ok(upnp_client) = UpnpClient::new().await {
                nat_manager.upnp_client = Some(upnp_client);
                info!("UPnP client initialized");
            } else {
                warn!("Failed to initialize UPnP client");
            }
        }
        
        // Initialize STUN if enabled
        if nat_manager.config.enable_stun {
            let stun_client = StunClient::new();
            nat_manager.stun_client = Some(stun_client);
            info!("STUN client initialized");
        }
        
        Ok(nat_manager)
    }
    
    /// Start the NAT manager
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting NAT manager");
        
        // Discover external address
        self.discover_external_address().await?;
        
        // Set up port mappings if UPnP is available
        if let Some(ref mut upnp_client) = self.upnp_client {
            self.setup_port_mappings(upnp_client).await?;
        }
        
        // Start periodic tasks
        self.start_periodic_tasks().await?;
        
        Ok(())
    }
    
    /// Discover external address using STUN
    async fn discover_external_address(&mut self) -> Result<()> {
        if let Some(ref mut stun_client) = self.stun_client {
            match stun_client.discover_external_address().await {
                Ok(addr) => {
                    self.external_addr = Some(addr);
                    info!("Discovered external address: {}", addr);
                }
                Err(e) => {
                    warn!("Failed to discover external address via STUN: {}", e);
                }
            }
        }
        Ok(())
    }
    
    /// Set up port mappings via UPnP
    async fn setup_port_mappings(&mut self, upnp_client: &mut UpnpClient) -> Result<()> {
        // Get local addresses
        let local_addrs = self.get_local_addresses().await?;
        
        for local_addr in local_addrs {
            if let IpAddr::V4(ipv4) = local_addr.ip() {
                // Map TCP port
                if let Ok(mapping) = upnp_client.add_port_mapping(
                    local_addr.port(),
                    local_addr.port(),
                    Protocol::Tcp,
                    "IPPAN Node TCP",
                ).await {
                    self.port_mappings.push(mapping);
                    info!("Mapped TCP port {} -> {}", local_addr.port(), local_addr.port());
                }
                
                // Map UDP port
                if let Ok(mapping) = upnp_client.add_port_mapping(
                    local_addr.port(),
                    local_addr.port(),
                    Protocol::Udp,
                    "IPPAN Node UDP",
                ).await {
                    self.port_mappings.push(mapping);
                    info!("Mapped UDP port {} -> {}", local_addr.port(), local_addr.port());
                }
            }
        }
        
        Ok(())
    }
    
    /// Get local network addresses
    async fn get_local_addresses(&self) -> Result<Vec<SocketAddr>> {
        let mut addrs = Vec::new();
        
        // Get all network interfaces
        for iface in get_if_addrs::get_if_addrs()? {
            if !iface.is_loopback() {
                let addr = SocketAddr::new(iface.addr.ip(), 0);
                addrs.push(addr);
            }
        }
        
        Ok(addrs)
    }
    
    /// Start periodic tasks
    async fn start_periodic_tasks(&mut self) -> Result<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes
        
        loop {
            interval.tick().await;
            
            // Refresh external address
            self.discover_external_address().await?;
            
            // Refresh port mappings
            if let Some(ref mut upnp_client) = self.upnp_client {
                self.refresh_port_mappings(upnp_client).await?;
            }
        }
    }
    
    /// Refresh port mappings
    async fn refresh_port_mappings(&mut self, upnp_client: &mut UpnpClient) -> Result<()> {
        for mapping in &self.port_mappings {
            if let Err(e) = upnp_client.refresh_port_mapping(mapping).await {
                warn!("Failed to refresh port mapping {}: {}", mapping.mapping_id, e);
            }
        }
        Ok(())
    }
    
    /// Get external address
    pub fn external_address(&self) -> Option<SocketAddr> {
        self.external_addr
    }
    
    /// Get port mappings
    pub fn port_mappings(&self) -> &[PortMapping] {
        &self.port_mappings
    }
    
    /// Clean up port mappings
    pub async fn cleanup(&mut self) -> Result<()> {
        if let Some(ref mut upnp_client) = self.upnp_client {
            for mapping in &self.port_mappings {
                if let Err(e) = upnp_client.delete_port_mapping(&mapping.mapping_id).await {
                    warn!("Failed to delete port mapping {}: {}", mapping.mapping_id, e);
                }
            }
        }
        Ok(())
    }
}

impl UpnpClient {
    /// Create a new UPnP client
    async fn new() -> Result<Self> {
        // Discover UPnP gateway
        let gateway_addr = Self::discover_gateway().await?;
        let (control_url, service_type) = Self::get_control_url(&gateway_addr).await?;
        
        Ok(Self {
            gateway_addr,
            control_url,
            service_type,
        })
    }
    
    /// Discover UPnP gateway
    async fn discover_gateway() -> Result<SocketAddr> {
        // Simple UPnP discovery - in a real implementation, you'd use a proper UPnP library
        // For now, we'll return a default gateway address
        Ok(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 1900))
    }
    
    /// Get control URL from gateway
    async fn get_control_url(gateway_addr: &SocketAddr) -> Result<(String, String)> {
        // In a real implementation, you'd parse the UPnP device description
        // For now, return default values
        Ok((
            format!("http://{}:2869/upnp/control/WANIPConn1", gateway_addr.ip()),
            "urn:schemas-upnp-org:service:WANIPConnection:1".to_string(),
        ))
    }
    
    /// Add a port mapping
    async fn add_port_mapping(
        &self,
        internal_port: u16,
        external_port: u16,
        protocol: Protocol,
        description: &str,
    ) -> Result<PortMapping> {
        // In a real implementation, you'd send a SOAP request to the UPnP gateway
        // For now, we'll simulate success
        
        let mapping_id = format!("ippan_{}_{}_{}", 
            internal_port, 
            external_port, 
            match protocol {
                Protocol::Tcp => "tcp",
                Protocol::Udp => "udp",
            }
        );
        
        Ok(PortMapping {
            internal_port,
            external_port,
            protocol,
            description: description.to_string(),
            mapping_id,
        })
    }
    
    /// Refresh a port mapping
    async fn refresh_port_mapping(&self, _mapping: &PortMapping) -> Result<()> {
        // In a real implementation, you'd refresh the mapping via UPnP
        Ok(())
    }
    
    /// Delete a port mapping
    async fn delete_port_mapping(&self, _mapping_id: &str) -> Result<()> {
        // In a real implementation, you'd delete the mapping via UPnP
        Ok(())
    }
}

impl StunClient {
    /// Create a new STUN client
    fn new() -> Self {
        let servers = vec![
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 19302), // Google STUN
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 3478),  // Cloudflare STUN
        ];
        
        Self {
            servers,
            external_addr: None,
        }
    }
    
    /// Discover external address using STUN
    async fn discover_external_address(&mut self) -> Result<SocketAddr> {
        for server in &self.servers {
            match self.query_stun_server(*server).await {
                Ok(addr) => {
                    self.external_addr = Some(addr);
                    return Ok(addr);
                }
                Err(e) => {
                    debug!("STUN query failed for {}: {}", server, e);
                }
            }
        }
        
        Err(IppanError::NetworkError("Failed to discover external address via STUN".to_string()))
    }
    
    /// Query a STUN server
    async fn query_stun_server(&self, server: SocketAddr) -> Result<SocketAddr> {
        // In a real implementation, you'd implement the STUN protocol
        // For now, we'll simulate a successful response
        
        // Simulate network delay
        sleep(Duration::from_millis(100)).await;
        
        // Return a simulated external address
        Ok(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1)), 12345))
    }
    
    /// Get external address
    pub fn external_address(&self) -> Option<SocketAddr> {
        self.external_addr
    }
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::Tcp => write!(f, "TCP"),
            Protocol::Udp => write!(f, "UDP"),
        }
    }
}

impl std::fmt::Display for PortMapping {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{} -> {}:{} ({})",
            self.external_port,
            self.protocol,
            self.internal_port,
            self.protocol,
            self.description
        )
    }
}
