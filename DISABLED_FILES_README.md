# Disabled Files Documentation

This document tracks files that have been temporarily disabled and explains why.

## Overview

Several example and test files have been disabled (renamed with `.disabled` extension) to prevent compilation issues or outdated functionality from blocking development.

## Disabled Files

### RPC Examples

#### `crates/rpc/examples/simple_server.rs.disabled`
- **Reason**: Example may reference outdated API or missing dependencies
- **To Re-enable**: 
  1. Verify all imports and dependencies are current
  2. Update configuration structs if API has changed
  3. Test that the server starts without errors
  4. Rename back to `.rs` extension

### Consensus Tests

#### `crates/consensus/tests/ai_consensus_integration_tests.rs.disabled`
- **Reason**: Integration tests may require specific AI models or test fixtures
- **Size**: 14KB (substantial test suite)
- **To Re-enable**:
  1. Ensure AI models are available for testing
  2. Update test fixtures if data format has changed
  3. Verify all test dependencies are installed
  4. Run tests individually to identify failures
  5. Rename back to `.rs` extension

#### `crates/consensus/tests/emission_integration.rs.disabled`
- **Reason**: Emission calculation tests may reference old economics parameters
- **Size**: 8.4KB
- **To Re-enable**:
  1. Update emission parameters to match current economics
  2. Verify emission calculation formulas are current
  3. Update expected values in assertions
  4. Rename back to `.rs` extension

### Wallet Examples

#### `crates/wallet/examples/advanced_usage.rs.disabled`
- **Reason**: Advanced wallet features may be under development
- **Size**: 9.7KB
- **To Re-enable**:
  1. Ensure all advanced wallet features are implemented
  2. Update example to use current wallet API
  3. Test all demonstrated features
  4. Rename back to `.rs` extension

#### `crates/wallet/examples/basic_usage.rs.disabled`
- **Reason**: Basic examples may reference deprecated wallet API
- **Size**: 3.6KB
- **To Re-enable**:
  1. Update to current wallet API
  2. Verify basic operations work as expected
  3. Update documentation comments
  4. Rename back to `.rs` extension

## Re-enabling Process

To re-enable any disabled file:

1. **Review the file**: Understand what it does and why it was disabled
2. **Update dependencies**: Ensure all imports and dependencies are current
3. **Fix API changes**: Update any outdated API calls
4. **Test thoroughly**: Run the example/test to ensure it works
5. **Update documentation**: Ensure comments and docs are accurate
6. **Rename file**: Remove the `.disabled` extension
7. **Commit changes**: Document what was fixed in the commit message

## Maintenance Policy

- **Review quarterly**: Check if disabled files can be re-enabled
- **Document clearly**: Always document why a file is disabled
- **Consider deletion**: If a file has been disabled for > 6 months and is no longer relevant, consider deleting it
- **Keep useful code**: Even if disabled, code can serve as reference or be revived later

## Status Tracking

| File | Disabled Since | Priority | Next Review |
|------|----------------|----------|-------------|
| simple_server.rs | 2024-11-03 | Low | 2025-02-04 |
| ai_consensus_integration_tests.rs | 2024-11-03 | High | 2025-01-04 |
| emission_integration.rs | 2024-11-03 | High | 2025-01-04 |
| advanced_usage.rs | 2024-11-03 | Medium | 2025-01-04 |
| basic_usage.rs | 2024-11-03 | High | 2024-12-04 |

## Notes

- **High Priority**: Should be re-enabled soon as they provide important functionality
- **Medium Priority**: Useful but not critical
- **Low Priority**: Nice-to-have examples or deprecated functionality

---

**Last Updated**: 2025-11-04  
**Maintainer**: IPPAN Development Team
