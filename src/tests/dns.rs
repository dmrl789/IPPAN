#[cfg(test)]
mod tests {
    use super::*;
    use crate::dns::resolver::ZoneResolver;
    use crate::dns::types::*;
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_ippan_naming_convention() {
        // Create a test zone
        let mut zones = BTreeMap::new();
        let mut alice_zone = Zone {
            domain: "alice.ipn".to_string(),
            owner_pk: [1u8; 32],
            version: 1,
            serial: 1,
            updated_at_us: 1234567890,
            rrsets: BTreeMap::new(),
        };

        // Add a test record
        let test_record = Rrset {
            ttl: 3600,
            records: vec![
                Record::A {
                    address: "192.168.1.100".to_string(),
                },
            ],
        };

        alice_zone.rrsets.insert(
            RrsetKey {
                name: "@".to_string(),
                rtype: Rtype::A,
            },
            test_record,
        );

        zones.insert("alice.ipn".to_string(), alice_zone);

        let resolver = ZoneResolver::new(Arc::new(RwLock::new(zones)));

        // Test IPPAN naming convention
        let result = resolver.resolve("ipn.alice.ipn", Rtype::A).await.unwrap();
        assert!(result.is_some());

        // Test that non-IPPAN names still work
        let result = resolver.resolve("alice.ipn", Rtype::A).await.unwrap();
        assert!(result.is_some());

        // Test that non-IPPAN names return None for non-existent domains
        let result = resolver.resolve("bob.ipn", Rtype::A).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_ippan_name_detection() {
        let zones = BTreeMap::new();
        let resolver = ZoneResolver::new(Arc::new(RwLock::new(zones)));

        // Test IPPAN name detection
        assert!(resolver.is_ipn_name("ipn.alice.ipn"));
        assert!(resolver.is_ipn_name("ipn.dao.fin"));
        assert!(resolver.is_ipn_name("ipn.music.amigo"));

        // Test non-IPPAN names
        assert!(!resolver.is_ipn_name("alice.ipn"));
        assert!(!resolver.is_ipn_name("www.example.com"));
        assert!(!resolver.is_ipn_name("api.alice.ipn"));
    }

    #[tokio::test]
    async fn test_ippan_name_resolution() {
        // Create a test zone
        let mut zones = BTreeMap::new();
        let mut dao_zone = Zone {
            domain: "dao.fin".to_string(),
            owner_pk: [2u8; 32],
            version: 1,
            serial: 1,
            updated_at_us: 1234567890,
            rrsets: BTreeMap::new(),
        };

        // Add a test record
        let test_record = Rrset {
            ttl: 3600,
            records: vec![
                Record::TXT {
                    text: "IPPAN DAO Dashboard".to_string(),
                },
            ],
        };

        dao_zone.rrsets.insert(
            RrsetKey {
                name: "@".to_string(),
                rtype: Rtype::TXT,
            },
            test_record,
        );

        zones.insert("dao.fin".to_string(), dao_zone);

        let resolver = ZoneResolver::new(Arc::new(RwLock::new(zones)));

        // Test IPPAN name resolution
        let result = resolver.resolve_ipn_name("ipn.dao.fin", Rtype::TXT).await.unwrap();
        assert!(result.is_some());

        // Test that non-IPPAN names return error
        let result = resolver.resolve_ipn_name("dao.fin", Rtype::TXT).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_service_subdomains() {
        // Create a test zone
        let mut zones = BTreeMap::new();
        let mut alice_zone = Zone {
            domain: "alice.ipn".to_string(),
            owner_pk: [1u8; 32],
            version: 1,
            serial: 1,
            updated_at_us: 1234567890,
            rrsets: BTreeMap::new(),
        };

        // Add API record
        let api_record = Rrset {
            ttl: 3600,
            records: vec![
                Record::A {
                    address: "192.168.1.101".to_string(),
                },
            ],
        };

        // Add CDN record
        let cdn_record = Rrset {
            ttl: 3600,
            records: vec![
                Record::A {
                    address: "192.168.1.102".to_string(),
                },
            ],
        };

        alice_zone.rrsets.insert(
            RrsetKey {
                name: "api".to_string(),
                rtype: Rtype::A,
            },
            api_record,
        );

        alice_zone.rrsets.insert(
            RrsetKey {
                name: "cdn".to_string(),
                rtype: Rtype::A,
            },
            cdn_record,
        );

        zones.insert("alice.ipn".to_string(), alice_zone);

        let resolver = ZoneResolver::new(Arc::new(RwLock::new(zones)));

        // Test service subdomain resolution
        let api_result = resolver.resolve("ipn.api.alice.ipn", Rtype::A).await.unwrap();
        assert!(api_result.is_some());

        let cdn_result = resolver.resolve("ipn.cdn.alice.ipn", Rtype::A).await.unwrap();
        assert!(cdn_result.is_some());
    }
}
