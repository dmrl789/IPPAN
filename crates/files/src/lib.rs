//! IPPAN File Descriptor System
//!
//! Provides metadata tracking and DHT-based discovery for content-addressed files.
//! File descriptors use HashTimer-based IDs for ordering and contain content hashes,
//! owner information, and metadata without storing the actual file content.

pub mod descriptor;
pub mod dht;
pub mod storage;

pub use descriptor::{FileDescriptor, FileId};
pub use dht::{DhtLookupResult, DhtPublishResult, FileDhtService};
pub use storage::{FileStorage, MemoryFileStorage};

#[cfg(test)]
mod tests;
