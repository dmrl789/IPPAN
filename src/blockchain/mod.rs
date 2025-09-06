#[cfg(feature = "contracts")]
pub mod smart_contract_system;

#[cfg(feature = "contracts")]
pub use smart_contract_system::VmHost;

 