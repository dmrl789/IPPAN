//! DNS types for IPPAN on-chain DNS system

use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;

/// Zone object containing DNS records
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zone {
    /// Domain name (e.g., "example.ipn")
    pub domain: String,
    /// Domain owner public key
    pub owner_pk: [u8; 32],
    /// Schema version
    pub version: u32,
    /// Auto-incrementing serial number
    pub serial: u32,
    /// Last update timestamp in microseconds
    pub updated_at_us: u64,
    /// DNS record sets keyed by name+type
    pub rrsets: BTreeMap<RrsetKey, Rrset>,
}

/// Key for identifying a record set
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RrsetKey {
    /// Record name (e.g., "@", "www", "_xmpp._tcp")
    pub name: String,
    /// Record type
    pub rtype: Rtype,
}

/// DNS record set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rrset {
    /// Time to live in seconds
    pub ttl: u32,
    /// Record values (validated against rtype)
    pub records: Vec<serde_json::Value>,
}

/// DNS record types
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Rtype {
    // Core address & name mapping
    A,           // IPv4 address
    AAAA,        // IPv6 address
    CNAME,       // Canonical name
    ALIAS,       // Apex alias (pseudo-type)
    
    // Mail & text
    MX,          // Mail exchange
    TXT,         // Text records
    SPF,         // Sender Policy Framework (legacy)
    
    // Nameservers & zone meta
    NS,          // Name server
    SOA,         // Start of authority
    
    // Service discovery
    SRV,         // Service record
    SVCB,        // Service binding
    HTTPS,       // HTTPS service binding
    
    // Security, certificates, SSH
    CAA,         // Certification Authority Authorization
    TLSA,        // TLS authentication
    SSHFP,       // SSH fingerprint
    
    // DNSSEC
    DNSKEY,      // DNS key
    DS,          // Delegation signer
    CDS,         // Child delegation signer
    CDNSKEY,     // Child DNS key
    
    // Reverse & telephony
    PTR,         // Pointer record
    NAPTR,       // Naming authority pointer
    
    // Web3 / content pointers
    CONTENT,     // Content-addressed pointers (pseudo-type)
}

impl Rtype {
    /// Get the string representation of the record type
    pub fn as_str(&self) -> &'static str {
        match self {
            Rtype::A => "A",
            Rtype::AAAA => "AAAA",
            Rtype::CNAME => "CNAME",
            Rtype::ALIAS => "ALIAS",
            Rtype::MX => "MX",
            Rtype::TXT => "TXT",
            Rtype::SPF => "SPF",
            Rtype::NS => "NS",
            Rtype::SOA => "SOA",
            Rtype::SRV => "SRV",
            Rtype::SVCB => "SVCB",
            Rtype::HTTPS => "HTTPS",
            Rtype::CAA => "CAA",
            Rtype::TLSA => "TLSA",
            Rtype::SSHFP => "SSHFP",
            Rtype::DNSKEY => "DNSKEY",
            Rtype::DS => "DS",
            Rtype::CDS => "CDS",
            Rtype::CDNSKEY => "CDNSKEY",
            Rtype::PTR => "PTR",
            Rtype::NAPTR => "NAPTR",
            Rtype::CONTENT => "CONTENT",
        }
    }

    /// Check if this record type is exclusive (cannot coexist with others at same name)
    pub fn is_exclusive(&self) -> bool {
        matches!(self, Rtype::CNAME | Rtype::ALIAS)
    }

    /// Check if this record type is allowed at apex
    pub fn allowed_at_apex(&self) -> bool {
        !matches!(self, Rtype::CNAME)
    }

    /// Get minimum TTL for this record type
    pub fn min_ttl(&self) -> u32 {
        match self {
            Rtype::SOA => 300,    // SOA has longer minimum
            _ => 60,              // Standard minimum
        }
    }

    /// Get maximum TTL for this record type
    pub fn max_ttl(&self) -> u32 {
        match self {
            Rtype::SOA => 86400,  // SOA has shorter maximum
            _ => 86400,           // Standard maximum
        }
    }
}

impl std::fmt::Display for Rtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for Rtype {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "A" => Ok(Rtype::A),
            "AAAA" => Ok(Rtype::AAAA),
            "CNAME" => Ok(Rtype::CNAME),
            "ALIAS" => Ok(Rtype::ALIAS),
            "MX" => Ok(Rtype::MX),
            "TXT" => Ok(Rtype::TXT),
            "SPF" => Ok(Rtype::SPF),
            "NS" => Ok(Rtype::NS),
            "SOA" => Ok(Rtype::SOA),
            "SRV" => Ok(Rtype::SRV),
            "SVCB" => Ok(Rtype::SVCB),
            "HTTPS" => Ok(Rtype::HTTPS),
            "CAA" => Ok(Rtype::CAA),
            "TLSA" => Ok(Rtype::TLSA),
            "SSHFP" => Ok(Rtype::SSHFP),
            "DNSKEY" => Ok(Rtype::DNSKEY),
            "DS" => Ok(Rtype::DS),
            "CDS" => Ok(Rtype::CDS),
            "CDNSKEY" => Ok(Rtype::CDNSKEY),
            "PTR" => Ok(Rtype::PTR),
            "NAPTR" => Ok(Rtype::NAPTR),
            "CONTENT" => Ok(Rtype::CONTENT),
            _ => Err(format!("Unknown record type: {}", s)),
        }
    }
}

/// DNS resolution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsResponse {
    /// Query name
    pub name: String,
    /// Query type
    pub rtype: Rtype,
    /// Record set (if found)
    pub rrset: Option<Rrset>,
    /// Response code
    pub rcode: ResponseCode,
    /// Authority records (for delegation)
    pub authority: Option<Rrset>,
}

/// DNS response codes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseCode {
    /// No error
    NoError,
    /// Name not found
    NXDomain,
    /// Not implemented
    NotImp,
    /// Server failure
    ServFail,
    /// Format error
    FormErr,
}

impl std::fmt::Display for ResponseCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResponseCode::NoError => write!(f, "NOERROR"),
            ResponseCode::NXDomain => write!(f, "NXDOMAIN"),
            ResponseCode::NotImp => write!(f, "NOTIMP"),
            ResponseCode::ServFail => write!(f, "SERVFAIL"),
            ResponseCode::FormErr => write!(f, "FORMERR"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rtype_display() {
        assert_eq!(Rtype::A.to_string(), "A");
        assert_eq!(Rtype::AAAA.to_string(), "AAAA");
        assert_eq!(Rtype::CNAME.to_string(), "CNAME");
    }

    #[test]
    fn test_rtype_from_str() {
        assert_eq!("A".parse::<Rtype>().unwrap(), Rtype::A);
        assert_eq!("AAAA".parse::<Rtype>().unwrap(), Rtype::AAAA);
        assert_eq!("CNAME".parse::<Rtype>().unwrap(), Rtype::CNAME);
        assert!("INVALID".parse::<Rtype>().is_err());
    }

    #[test]
    fn test_rtype_exclusive() {
        assert!(Rtype::CNAME.is_exclusive());
        assert!(Rtype::ALIAS.is_exclusive());
        assert!(!Rtype::A.is_exclusive());
        assert!(!Rtype::TXT.is_exclusive());
    }

    #[test]
    fn test_rtype_apex_allowed() {
        assert!(!Rtype::CNAME.allowed_at_apex());
        assert!(Rtype::ALIAS.allowed_at_apex());
        assert!(Rtype::A.allowed_at_apex());
        assert!(Rtype::TXT.allowed_at_apex());
    }

    #[test]
    fn test_zone_creation() {
        let zone = Zone {
            domain: "example.ipn".to_string(),
            owner_pk: [0u8; 32],
            version: 1,
            serial: 1,
            updated_at_us: 0,
            rrsets: BTreeMap::new(),
        };
        
        assert_eq!(zone.domain, "example.ipn");
        assert_eq!(zone.serial, 1);
        assert!(zone.rrsets.is_empty());
    }
}
