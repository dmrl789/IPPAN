pub enum IpTxtType {
    FileDescription,  // e.g., summary of a PDF or dataset
    ServerInfo,       // service availability and endpoint metadata
    DNSLikeRecord,    // domain=xyz.com; tls_sha256=...
    ProofBinding,     // Signed declaration of handle ↔ resource link
}

pub struct IpTxtRecord {
    pub record_type: IpTxtType,
    pub handle: String,           // e.g. @alice.ipn
    pub content: String,          // human-readable or key=value; max ~512 bytes
    pub timestamp: HashTimer,
    pub signature: String,        // ed25519 sig over (handle + content + timestamp)
}

impl IpTxtRecord {
    pub fn new(record_type: IpTxtType, handle: String, content: String, timestamp: HashTimer, signature: String) -> Self {
        IpTxtRecord {
            record_type,
            handle,
            content,
            timestamp,
            signature,
        }
    }
} 