//! IPPAN File Descriptor System
//!
//! Provides metadata tracking and DHT-based discovery for content-addressed files.
//! File descriptors use HashTimer-based IDs for ordering and contain content hashes,
//! owner information, and metadata without storing the actual file content.

pub mod descriptor;
pub mod storage;
pub mod dht;

pub use descriptor::{FileDescriptor, FileId};
pub use storage::{FileStorage, MemoryFileStorage};
pub use dht::{FileDhtService, DhtPublishResult, DhtLookupResult};

#[cfg(test)]
mod tests;
