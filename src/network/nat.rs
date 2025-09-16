//! NAT traversal service for IPPAN network

use crate::Result;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

/// NAT traversal service
#[derive(Debug)]
pub struct NATService {
    /// External IP address
    external_ip: Option<IpAddr>,
    /// External port
    external_port: Option<u16>,
    /// NAT type
    nat_type: NATType,
    /// UPnP enabled
    upnp_enabled: bool,
    /// STUN servers
    stun_servers: Vec<String>,
    /// Running flag
    running: bool,
}

impl Default for NATService {
    fn default() -> Self {
        Self {
            external_ip: None,
            external_port: None,
            nat_type: NATType::Unknown,
            upnp_enabled: false,
            stun_servers: Vec::new(),
            running: false,
        }
    }
}

/// NAT type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NATType {
    /// No NAT
    Open,
    /// Full cone NAT
    FullCone,
    /// Restricted cone NAT
    RestrictedCone,
    /// Port restricted cone NAT
    PortRestrictedCone,
    /// Symmetric NAT
    Symmetric,
    /// Unknown
    Unknown,
}

impl NATService {
    /// Create a new NAT service
    pub fn new() -> Self {
        Self {
            external_ip: None,
            external_port: None,
            nat_type: NATType::Unknown,
            upnp_enabled: false,
            stun_servers: vec![
                "stun.l.google.com:19302".to_string(),
                "stun1.l.google.com:19302".to_string(),
            ],
            running: false,
        }
    }

    /// Start the NAT service
    pub async fn start(&mut self) -> Result<()> {
        log::info!("NAT service temporarily disabled");
        return Ok(());
        // log::info!("Starting NAT traversal service");
        // self.running = true;
        
        // Discover external IP
        self.discover_external_ip().await?;
        
        // Determine NAT type
        self.determine_nat_type().await?;
        
        // Try UPnP port mapping
        if self.upnp_enabled {
            self.try_upnp_mapping().await?;
        }
        
        Ok(())
    }

    /// Stop the NAT service
    pub async fn stop(&mut self) -> Result<()> {
        log::info!("Stopping NAT traversal service");
        self.running = false;
        
        // Remove UPnP port mappings
        if self.upnp_enabled {
            self.remove_upnp_mapping().await?;
        }
        
        Ok(())
    }

    /// Discover external IP address
    async fn discover_external_ip(&mut self) -> Result<()> {
        log::info!("Discovering external IP address...");
        
        // Try STUN servers
        for stun_server in &self.stun_servers {
            if let Ok(ip) = self.query_stun_server(stun_server).await {
                self.external_ip = Some(ip);
                log::info!("Discovered external IP: {}", ip);
                return Ok(());
            }
        }
        
        // Fallback to HTTP-based IP discovery
        if let Ok(ip) = self.query_http_ip_service().await {
            self.external_ip = Some(ip);
            log::info!("Discovered external IP via HTTP: {}", ip);
            return Ok(());
        }
        
        log::warn!("Failed to discover external IP address");
        Ok(())
    }

    /// Determine NAT type
    async fn determine_nat_type(&mut self) -> Result<()> {
        log::info!("Determining NAT type...");
        
        // TODO: Implement NAT type detection
        // This would involve multiple STUN queries with different source ports
        // and analyzing the responses to determine the NAT behavior
        
        self.nat_type = NATType::Unknown;
        log::info!("NAT type: {:?}", self.nat_type);
        
        Ok(())
    }

    /// Try UPnP port mapping
    async fn try_upnp_mapping(&mut self) -> Result<()> {
        log::info!("Attempting UPnP port mapping...");
        
        // Discover UPnP devices
        let devices = self.discover_upnp_devices().await?;
        
        if devices.is_empty() {
            log::warn!("No UPnP devices found on network");
            return Ok(());
        }
        
        // Try to add port mapping on each device
        for device in devices {
            if let Ok(()) = self.add_upnp_port_mapping(&device, 8333, 8333).await {
                log::info!("Successfully added UPnP port mapping on device: {}", device.control_url);
                self.external_port = Some(8333);
                return Ok(());
            }
        }
        
        log::warn!("Failed to add UPnP port mapping on any device");
        Ok(())
    }
    
    /// Discover UPnP devices using SSDP
    async fn discover_upnp_devices(&self) -> Result<Vec<UpnpDevice>> {
        let mut devices = Vec::new();
        
        // SSDP M-SEARCH request
        let search_request = [
            "M-SEARCH * HTTP/1.1",
            "HOST: 239.255.255.250:1900",
            "MAN: \"ssdp:discover\"",
            "MX: 3",
            "ST: urn:schemas-upnp-org:device:InternetGatewayDevice:1",
            "",
            "",
        ].join("\r\n");
        
        // Create UDP socket for SSDP
        let socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await
            .map_err(|e| crate::error::IppanError::Network(
                format!("Failed to create SSDP socket: {}", e)
            ))?;
        
        // Send M-SEARCH
        let ssdp_addr = "239.255.255.250:1900".parse::<std::net::SocketAddr>()
            .map_err(|e| crate::error::IppanError::Network(
                format!("Invalid SSDP address: {}", e)
            ))?;
        
        socket.send_to(search_request.as_bytes(), ssdp_addr).await
            .map_err(|e| crate::error::IppanError::Network(
                format!("Failed to send SSDP M-SEARCH: {}", e)
            ))?;
        
        // Listen for responses
        let mut buffer = [0u8; 1024];
        let timeout = tokio::time::sleep(std::time::Duration::from_secs(5));
        tokio::pin!(timeout);
        let mut responses = Vec::new();
        
        loop {
            tokio::select! {
                result = socket.recv_from(&mut buffer) => {
                    match result {
                        Ok((len, _)) => {
                            let response = String::from_utf8_lossy(&buffer[..len]);
                            responses.push(response.to_string());
                        }
                        Err(_) => break,
                    }
                }
                _ = &mut timeout => break,
            }
        }
        
        // Parse responses to extract device information
        for response in responses {
            if let Some(device) = self.parse_ssdp_response(&response) {
                devices.push(device);
            }
        }
        
        Ok(devices)
    }
    
    /// Parse SSDP response to extract device information
    fn parse_ssdp_response(&self, response: &str) -> Option<UpnpDevice> {
        let lines: Vec<&str> = response.lines().collect();
        
        // Look for LOCATION header
        let location = lines.iter()
            .find(|line| line.starts_with("LOCATION:"))
            .and_then(|line| line.split_once(':'))
            .map(|(_, value)| value.trim())?;
        
        // For now, create a basic device structure
        // In a full implementation, we would fetch and parse the device description XML
        Some(UpnpDevice {
            control_url: location.to_string(),
            device_type: "InternetGatewayDevice".to_string(),
        })
    }
    
    /// Add UPnP port mapping
    async fn add_upnp_port_mapping(&self, device: &UpnpDevice, external_port: u16, internal_port: u16) -> Result<()> {
        // SOAP request for AddPortMapping
        let soap_body = format!(
            r#"<?xml version="1.0"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
  <s:Body>
    <u:AddPortMapping xmlns:u="urn:schemas-upnp-org:service:WANIPConnection:1">
      <NewRemoteHost></NewRemoteHost>
      <NewExternalPort>{}</NewExternalPort>
      <NewProtocol>UDP</NewProtocol>
      <NewInternalPort>{}</NewInternalPort>
      <NewInternalClient>{}</NewInternalClient>
      <NewEnabled>1</NewEnabled>
      <NewPortMappingDescription>IPPAN Node</NewPortMappingDescription>
      <NewLeaseDuration>0</NewLeaseDuration>
    </u:AddPortMapping>
  </s:Body>
</s:Envelope>"#,
            external_port,
            internal_port,
            self.get_local_ip().unwrap_or_else(|| "192.168.1.100".to_string())
        );
        
        // Send SOAP request
        let client = reqwest::Client::new();
        let response = client.post(&device.control_url)
            .header("Content-Type", "text/xml; charset=\"utf-8\"")
            .header("SOAPAction", "\"urn:schemas-upnp-org:service:WANIPConnection:1#AddPortMapping\"")
            .body(soap_body)
            .send()
            .await
            .map_err(|e| crate::error::IppanError::Network(
                format!("UPnP SOAP request failed: {}", e)
            ))?;
        
        if response.status().is_success() {
            log::info!("UPnP port mapping added successfully");
            Ok(())
        } else {
            Err(crate::error::IppanError::Network(
                format!("UPnP port mapping failed with status: {}", response.status())
            ))
        }
    }
    
    /// Get local IP address
    fn get_local_ip(&self) -> Option<String> {
        // Simple implementation - get first non-loopback IP
        for interface in if_addrs::get_if_addrs().ok()? {
            if !interface.is_loopback() && interface.addr.ip().is_ipv4() {
                return Some(interface.addr.ip().to_string());
            }
        }
        None
    }
    
    /// Remove UPnP port mapping
    async fn remove_upnp_mapping(&mut self) -> Result<()> {
        log::info!("Removing UPnP port mapping...");
        
        // TODO: Implement UPnP port mapping removal
        // This would involve discovering devices again and sending DeletePortMapping SOAP requests
        
        Ok(())
    }

    /// Query STUN server
    async fn query_stun_server(&self, server: &str) -> Result<IpAddr> {
        log::debug!("Querying STUN server: {}", server);
        
        // Parse server address
        let server_addr = server.parse::<std::net::SocketAddr>()
            .map_err(|e| crate::error::IppanError::Network(
                format!("Invalid STUN server address: {}", e)
            ))?;
        
        // Create UDP socket
        let socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await
            .map_err(|e| crate::error::IppanError::Network(
                format!("Failed to create UDP socket: {}", e)
            ))?;
        
        // Create STUN binding request
        let request = self.create_stun_binding_request();
        
        // Send request
        socket.send_to(&request, server_addr).await
            .map_err(|e| crate::error::IppanError::Network(
                format!("Failed to send STUN request: {}", e)
            ))?;
        
        // Receive response
        let mut buffer = [0u8; 1024];
        let (len, _) = socket.recv_from(&mut buffer).await
            .map_err(|e| crate::error::IppanError::Network(
                format!("Failed to receive STUN response: {}", e)
            ))?;
        
        // Parse STUN response
        self.parse_stun_response(&buffer[..len])
    }
    
    /// Create STUN binding request
    fn create_stun_binding_request(&self) -> Vec<u8> {
        // STUN message format:
        // 0-1: Message Type (Binding Request = 0x0001)
        // 2-3: Message Length
        // 4-7: Magic Cookie (0x2112A442)
        // 8-11: Transaction ID (random)
        
        let mut request = Vec::new();
        
        // Message Type: Binding Request
        request.extend_from_slice(&[0x00, 0x01]);
        
        // Message Length: 0 (no attributes)
        request.extend_from_slice(&[0x00, 0x00]);
        
        // Magic Cookie
        request.extend_from_slice(&[0x21, 0x12, 0xA4, 0x42]);
        
        // Transaction ID (96 bits = 12 bytes)
        let transaction_id: [u8; 12] = rand::random();
        request.extend_from_slice(&transaction_id);
        
        request
    }
    
    /// Parse STUN response
    fn parse_stun_response(&self, data: &[u8]) -> Result<IpAddr> {
        if data.len() < 20 {
            return Err(crate::error::IppanError::Network(
                "STUN response too short".to_string()
            ));
        }
        
        // Check message type (should be Binding Success Response = 0x0101)
        let message_type = u16::from_be_bytes([data[0], data[1]]);
        if message_type != 0x0101 {
            return Err(crate::error::IppanError::Network(
                format!("Unexpected STUN message type: 0x{:04x}", message_type)
            ));
        }
        
        // Check magic cookie
        let magic_cookie = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        if magic_cookie != 0x2112A442 {
            return Err(crate::error::IppanError::Network(
                "Invalid STUN magic cookie".to_string()
            ));
        }
        
        // Parse attributes to find XOR-MAPPED-ADDRESS
        let message_length = u16::from_be_bytes([data[2], data[3]]) as usize;
        let mut offset = 20; // Skip header
        
        while offset + 4 <= 20 + message_length {
            let attr_type = u16::from_be_bytes([data[offset], data[offset + 1]]);
            let attr_length = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;
            
            if offset + 4 + attr_length > data.len() {
                break;
            }
            
            // XOR-MAPPED-ADDRESS attribute (0x0020)
            if attr_type == 0x0020 && attr_length >= 8 {
                return self.parse_xor_mapped_address(&data[offset + 4..offset + 4 + attr_length]);
            }
            
            offset += 4 + attr_length;
        }
        
        Err(crate::error::IppanError::Network(
            "No XOR-MAPPED-ADDRESS found in STUN response".to_string()
        ))
    }
    
    /// Parse XOR-MAPPED-ADDRESS attribute
    fn parse_xor_mapped_address(&self, data: &[u8]) -> Result<IpAddr> {
        if data.len() < 8 {
            return Err(crate::error::IppanError::Network(
                "XOR-MAPPED-ADDRESS too short".to_string()
            ));
        }
        
        let family = data[1];
        let port = u16::from_be_bytes([data[2], data[3]]);
        
        match family {
            0x01 => { // IPv4
                if data.len() < 8 {
                    return Err(crate::error::IppanError::Network(
                        "IPv4 XOR-MAPPED-ADDRESS too short".to_string()
                    ));
                }
                
                // XOR with magic cookie
                let mut ip_bytes = [0u8; 4];
                for i in 0..4 {
                    ip_bytes[i] = data[4 + i] ^ 0x21 ^ 0x12 ^ 0xA4 ^ 0x42;
                }
                
                let ip = std::net::Ipv4Addr::from(ip_bytes);
                log::debug!("STUN discovered IPv4: {}:{}", ip, port);
                Ok(IpAddr::V4(ip))
            },
            0x02 => { // IPv6
                if data.len() < 20 {
                    return Err(crate::error::IppanError::Network(
                        "IPv6 XOR-MAPPED-ADDRESS too short".to_string()
                    ));
                }
                
                // XOR with magic cookie and transaction ID
                let mut ip_bytes = [0u8; 16];
                for i in 0..16 {
                    ip_bytes[i] = data[4 + i] ^ 0x21 ^ 0x12 ^ 0xA4 ^ 0x42;
                }
                
                let ip = std::net::Ipv6Addr::from(ip_bytes);
                log::debug!("STUN discovered IPv6: {}:{}", ip, port);
                Ok(IpAddr::V6(ip))
            },
            _ => Err(crate::error::IppanError::Network(
                format!("Unsupported address family: {}", family)
            ))
        }
    }

    /// Query HTTP IP service
    async fn query_http_ip_service(&self) -> Result<IpAddr> {
        log::debug!("Querying HTTP IP service");
        
        // List of IP discovery services to try
        let ip_services = vec![
            "https://api.ipify.org",
            "https://ipinfo.io/ip",
            "https://icanhazip.com",
            "https://ident.me",
        ];
        
        for service_url in ip_services {
            if let Ok(ip) = self.query_single_ip_service(service_url).await {
                return Ok(ip);
            }
        }
        
        Err(crate::error::IppanError::Network(
            "All HTTP IP services failed".to_string()
        ))
    }
    
    /// Query a single HTTP IP service
    async fn query_single_ip_service(&self, url: &str) -> Result<IpAddr> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| crate::error::IppanError::Network(
                format!("Failed to create HTTP client: {}", e)
            ))?;
        
        let response = client.get(url)
            .header("User-Agent", "IPPAN-NAT-Service/1.0")
            .send()
            .await
            .map_err(|e| crate::error::IppanError::Network(
                format!("HTTP request failed for {}: {}", url, e)
            ))?;
        
        if !response.status().is_success() {
            return Err(crate::error::IppanError::Network(
                format!("HTTP request failed with status: {}", response.status())
            ));
        }
        
        let ip_text = response.text().await
            .map_err(|e| crate::error::IppanError::Network(
                format!("Failed to read response body: {}", e)
            ))?;
        
        // Clean up the IP text (remove whitespace, newlines, etc.)
        let ip_text = ip_text.trim();
        
        // Parse the IP address
        ip_text.parse::<IpAddr>()
            .map_err(|e| crate::error::IppanError::Network(
                format!("Failed to parse IP address '{}': {}", ip_text, e)
            ))
    }

    /// Get external IP address
    pub fn get_external_ip(&self) -> Option<IpAddr> {
        self.external_ip
    }

    /// Get external port
    pub fn get_external_port(&self) -> Option<u16> {
        self.external_port
    }

    /// Get NAT type
    pub fn get_nat_type(&self) -> &NATType {
        &self.nat_type
    }

    /// Check if UPnP is enabled
    pub fn is_upnp_enabled(&self) -> bool {
        self.upnp_enabled
    }

    /// Enable UPnP
    pub fn enable_upnp(&mut self) {
        self.upnp_enabled = true;
    }

    /// Disable UPnP
    pub fn disable_upnp(&mut self) {
        self.upnp_enabled = false;
    }

    /// Add STUN server
    pub fn add_stun_server(&mut self, server: String) {
        self.stun_servers.push(server);
    }

    /// Get STUN servers
    pub fn get_stun_servers(&self) -> &[String] {
        &self.stun_servers
    }
}

/// UPnP device information
#[derive(Debug, Clone)]
struct UpnpDevice {
    control_url: String,
    device_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_nat_service_creation() {
        let service = NATService::new();
        
        assert_eq!(service.nat_type, NATType::Unknown);
        assert!(!service.upnp_enabled);
        assert!(!service.stun_servers.is_empty());
    }

    #[tokio::test]
    async fn test_nat_service_start_stop() {
        let mut service = NATService::new();
        
        service.start().await.unwrap();
        assert!(service.running);
        
        service.stop().await.unwrap();
        assert!(!service.running);
    }

    #[tokio::test]
    async fn test_upnp_control() {
        let mut service = NATService::new();
        
        assert!(!service.is_upnp_enabled());
        
        service.enable_upnp();
        assert!(service.is_upnp_enabled());
        
        service.disable_upnp();
        assert!(!service.is_upnp_enabled());
    }

    #[tokio::test]
    async fn test_stun_server_management() {
        let mut service = NATService::new();
        
        let initial_count = service.get_stun_servers().len();
        
        service.add_stun_server("stun.example.com:3478".to_string());
        
        assert_eq!(service.get_stun_servers().len(), initial_count + 1);
    }
}
