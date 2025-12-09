# Private Network Considerations for Devnet-1

## Current State

**Nodes on Private Network (10.0.0.0/24):**
- node1: 188.245.97.41 (public) / 10.0.0.2 (private)
- node2: 135.181.145.174 (public) / 10.0.0.3 (private)

**Nodes NOT on Private Network:**
- node3: 5.223.51.238 (public only)
- node4: 178.156.219.107 (public only)

## Impact

Currently, inter-node communication between node3/node4 and node1/node2 uses public IPs over the internet. This is:
- ✅ **Acceptable for devnet** - Functionality works correctly
- ⚠️ **Not ideal** - Adds latency and uses public bandwidth
- ✅ **Secure** - Firewall allowlist rules restrict RPC (8080) to devnet peers only

## Recommendation

**Future Enhancement:** Add node3 and node4 to the Hetzner private network (10.0.0.0/24) to:
- Reduce latency between nodes
- Reduce public bandwidth usage
- Improve network isolation

**Action Required:**
1. In Hetzner Cloud Console, add node3 and node4 to the private network
2. Assign private IPs (e.g., 10.0.0.4 for node3, 10.0.0.5 for node4)
3. Update bootstrap configs to use private IPs where beneficial
4. Keep current firewall allowlist rules in place (they work for both public and private IPs)

**Current Firewall Rules (Keep):**
- Allow from 10.0.0.0/24 (covers private network)
- Allow from specific public IPs (covers node3/node4 until they're on private net)

## Status

- ✅ Current setup is functional and secure
- 📋 Private network expansion is a future optimization, not a blocker

