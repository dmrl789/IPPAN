# Temporarily disabled files with floats

These modules are not currently compiled in the active build configuration:
- ai_consensus.rs → ai_consensus.rs.disabled (not in lib.rs module list)

These modules have floats but are behind feature flags:
- round.rs: Uses f64 under #[cfg(not(feature = "ai_l1"))]
- l1_ai_consensus.rs: Uses f64 for external API structs only

All ACTIVE runtime code has been migrated to fixed-point arithmetic.

