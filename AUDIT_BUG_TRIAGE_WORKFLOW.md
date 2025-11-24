# IPPAN External Audit - Bug Triage & Response Workflow

**Version:** 1.0  
**Date:** 2025-11-24  
**Owner:** Lead Architect + Security Lead  
**Purpose:** Define clear process for handling audit findings

---

## 1. Overview

This document establishes the **bug triage and response workflow** for findings from the external security audit. It ensures:

1. **Rapid response** to critical vulnerabilities
2. **Clear severity classification** for all findings
3. **Transparent tracking** of fixes and re-testing
4. **Systematic regression prevention** via test coverage

All findings are tracked in a private GitHub Security Advisory until mainnet launch.

---

## 2. Severity Classification

### 2.1 Severity Levels

| Severity | Impact | Response Time | Mainnet Blocker |
|----------|--------|---------------|-----------------|
| **Critical** | Direct loss of funds, consensus failure, network halt, remote code execution | **24 hours** | ✅ YES |
| **High** | Potential loss of funds, DoS attack, significant security bypass, validator manipulation | **72 hours** | ✅ YES |
| **Medium** | Logic errors, limited DoS, minor security impact, incorrect behavior under edge cases | **1 week** | ⚠️ DEPENDS |
| **Low** | Best practice violations, code quality issues, minor inefficiencies | **2 weeks** | ❌ NO |
| **Informational** | Documentation gaps, suggestions, non-security improvements | **Not binding** | ❌ NO |

### 2.2 Severity Criteria

#### Critical Severity

**Examples:**
- Supply cap violation (inflation attack)
- Consensus fork without recovery
- Private key exposure via API
- Remote code execution
- Network-wide halt (liveness failure)
- Double-spend vulnerability

**Impact:**
- Direct financial loss for users
- Chain integrity compromised
- Network unusable

**Response:**
- Immediate team mobilization (24/7 availability)
- Hot-fix patch development begins within 4 hours
- Public disclosure delayed until patch deployed (responsible disclosure)

---

#### High Severity

**Examples:**
- Validator reputation manipulation
- Eclipse attack vulnerability
- Transaction replay attack
- Signature malleability
- Fee bypassing
- Memory exhaustion DoS
- Emission rate manipulation (non-critical)

**Impact:**
- Potential financial loss (requires attacker sophistication)
- Network degradation (but not total failure)
- Security model weakened

**Response:**
- Patch development within 72 hours
- Emergency release if actively exploited
- Coordinated disclosure with auditors

---

#### Medium Severity

**Examples:**
- Non-exploitable logic errors
- Edge case handling issues
- Minor DoS (rate-limiting gaps)
- Incomplete input validation (low impact)
- Timestamp manipulation (non-critical)

**Impact:**
- Incorrect behavior in specific scenarios
- User experience degradation
- Limited security impact

**Response:**
- Fix in next scheduled release (within 1 week)
- Test coverage added for edge case
- May not block mainnet if workaround exists

---

#### Low Severity

**Examples:**
- Inefficient algorithms (non-critical paths)
- Missing error messages
- Code duplication
- Inconsistent naming
- Unused dependencies

**Impact:**
- Code quality/maintainability
- No security impact

**Response:**
- Addressed in regular development cycle (2 weeks)
- Does not block mainnet launch

---

#### Informational

**Examples:**
- Documentation suggestions
- Architecture recommendations
- Performance optimization ideas
- Best practice improvements

**Impact:**
- None (informational only)

**Response:**
- Noted for future consideration
- No immediate action required

---

## 3. Bug Triage Process

### 3.1 Initial Triage (Within 24 Hours)

**Step 1: Auditor Reports Finding**
- Auditor submits finding via secure channel (private GitHub Security Advisory)
- Includes: description, severity suggestion, PoC (if applicable), affected files

**Step 2: Internal Review**
- Lead Architect + Security Lead review finding
- Classify severity using criteria above
- Assign DRI (Directly Responsible Individual) from team

**Step 3: Confirm Severity with Auditor**
- Discuss severity classification with auditor
- Reach consensus (auditor has final say if dispute)
- Document agreed severity in tracking system

**Step 4: Create Tracking Issue**
- Create private GitHub issue in security advisory
- Label: `audit-finding`, `severity-critical/high/medium/low/info`
- Assign DRI + due date based on severity

---

### 3.2 Triage Template

```markdown
## Finding: [SHORT_TITLE]

**Severity:** [Critical/High/Medium/Low/Informational]  
**Reported By:** [Auditor Name]  
**Date Reported:** YYYY-MM-DD  
**DRI:** [Team Member]  
**Due Date:** YYYY-MM-DD (based on severity SLA)

### Description
[Auditor's description of the vulnerability]

### Proof of Concept
[PoC code/steps to reproduce, if provided]

### Affected Files
- `crates/[crate]/src/[file].rs:L123-L456`
- `crates/[crate]/tests/[test].rs`

### Impact Assessment
[Internal team assessment of actual impact]

### Proposed Fix
[High-level description of fix approach]

### Fix Commits
- [ ] Code fix: [commit hash / PR link]
- [ ] Test coverage: [commit hash / PR link]
- [ ] Documentation: [commit hash / PR link]

### Re-Test Status
- [ ] Unit test added (passes)
- [ ] Long-run simulation re-run (passes)
- [ ] Fuzz target updated (no crashes)
- [ ] Auditor reviewed fix (approved)

### Sign-Off
- [ ] DRI: [Name] - YYYY-MM-DD
- [ ] Lead Architect: [Name] - YYYY-MM-DD
- [ ] Auditor: [Name] - YYYY-MM-DD
```

---

## 4. Fix Development Workflow

### 4.1 Critical/High Severity Fixes

**Phase 1: Immediate Response (Hours 0-4)**
1. Team lead mobilizes DRI + relevant experts
2. Create private feature branch: `fix/audit-[finding-id]-[short-desc]`
3. Develop minimal fix (no refactoring, narrow scope)
4. Add unit test demonstrating fix

**Phase 2: Internal Validation (Hours 4-24)**
1. Run full test suite: `cargo test --workspace --release`
2. Re-run affected fuzz target: `cargo +nightly fuzz run [target] -- -max_total_time=600`
3. Re-run long-run DLC gate (if consensus-related): `cargo test -p ippan-consensus-dlc phase_e_long_run_dlc_gate --release -- --ignored --nocapture`
4. Code review by Lead Architect + 1 other team member

**Phase 3: Auditor Review (Hours 24-72)**
1. Share patch diff with auditor (via secure channel)
2. Auditor reviews fix + validates it addresses vulnerability
3. Auditor confirms no new issues introduced
4. If approved → merge to `master` (via fast-track process)
5. If rejected → iterate and re-submit

**Phase 4: Deployment (Hours 72+)**
1. Tag emergency release: `v0.x.y-audit-fix`
2. Deploy to internal testnet
3. Monitor for 24-48 hours
4. If stable → prepare for mainnet (coordinated with auditor)

---

### 4.2 Medium/Low Severity Fixes

**Less Urgent Process:**
1. Create fix branch (no emergency mobilization)
2. Develop fix with comprehensive test coverage
3. Optional refactoring/cleanup (if appropriate)
4. Standard code review process
5. Merge to `master` in next release cycle
6. Auditor reviews batch of fixes in scheduled re-audit session

---

## 5. Regression Prevention

### 5.1 Test Coverage Requirements

**Every fix MUST include:**

1. **Unit Test**
   - Demonstrates vulnerability is fixed
   - Fails on unfixed code, passes on fixed code
   - Located in same crate as fix (e.g., `crates/consensus/src/lib.rs` → `crates/consensus/tests/audit_fix_[id].rs`)

2. **Regression Test** (if applicable)
   - Broader integration test covering attack scenario
   - May be property-based test (via `proptest`)
   - May be fuzz target update (if parsing/validation related)

3. **Long-Run Gate Re-Run** (if consensus/economics related)
   - `phase_e_long_run_dlc_gate` must still pass
   - `phase_e_determinism_gate.sh` must produce same digest
   - Document pass/fail in fix tracking issue

---

### 5.2 Fuzz Target Updates

**If finding involves:**
- Parsing logic → Update relevant fuzz target (e.g., `fuzz_transaction_decode`, `fuzz_rpc_payment`)
- Consensus logic → Update `fuzz_consensus_round`
- P2P message handling → Update `fuzz_p2p_message`
- Crypto operations → Update `fuzz_crypto_signatures`

**Process:**
1. Add PoC input to fuzz corpus (if applicable)
2. Update fuzz target to explicitly test vulnerability scenario
3. Run fuzz target for 10 minutes: `cargo +nightly fuzz run [target] -- -max_total_time=600`
4. Verify no crashes/panics
5. Document in fix tracking issue

---

## 6. Re-Audit Process

### 6.1 Patch Submission

**After fixing all Critical/High findings:**

1. **Prepare Patch Package**
   - Git diff of all fixes: `git diff [audit-baseline-commit]..HEAD > audit-fixes.patch`
   - Summary document listing:
     - Finding ID → Fix commit hash
     - Test coverage added
     - Re-test results (gates, fuzz, unit tests)
   - Updated documentation (if applicable)

2. **Submit to Auditor**
   - Via secure channel (private GitHub repo or encrypted email)
   - Include: patch file, summary doc, test execution logs
   - Request re-audit within 1-2 weeks

3. **Auditor Re-Review**
   - Auditor validates each fix
   - Runs test suite independently
   - Confirms no new vulnerabilities introduced
   - Provides updated severity for any unfixed issues

---

### 6.2 Re-Audit Scope

**Full Re-Audit (if Critical/High fixes are extensive):**
- Re-run entire audit scope (consensus, crypto, network, economics)
- Duration: 2-4 weeks
- Required if >10 Critical/High findings or if major refactoring

**Targeted Re-Audit (if fixes are localized):**
- Review only affected components
- Duration: 1 week
- Acceptable if <5 Critical/High findings and fixes are narrow

**Auditor decides scope based on fix complexity.**

---

### 6.3 Final Sign-Off Criteria

**Mainnet launch requires:**

- ✅ All **Critical** findings resolved (auditor-approved)
- ✅ All **High** findings resolved (auditor-approved)
- ✅ ≥90% of **Medium** findings resolved (or documented as acceptable risk)
- ✅ Long-run DLC gate still passing (1200+ rounds, 6 invariants)
- ✅ Determinism gate still passing (BLAKE3 digest matches baseline)
- ✅ Full test suite passing (`cargo test --workspace`)
- ✅ All fuzz targets run for 10+ minutes without crashes
- ✅ Auditor provides final sign-off letter

**Medium findings may be deferred if:**
- Acceptable workaround exists
- Low exploitation likelihood
- Fix introduces more risk than bug itself
- Lead Architect + Auditor both agree to defer

---

## 7. Communication Plan

### 7.1 Internal Communication

**Critical/High Findings:**
- Immediate Slack alert to `#security-alerts` channel
- Daily standup updates until resolved
- Weekly report to all maintainers

**Medium/Low Findings:**
- Weekly digest email to team
- Tracked in regular sprint planning

---

### 7.2 Auditor Communication

**Weekly Sync (During Active Audit):**
- Video call: 1 hour
- Agenda:
  - Recent findings review
  - Clarifications on existing findings
  - Progress updates on fixes
  - Next week scope

**Ad-Hoc Communication:**
- Critical findings: Immediate Slack/Signal message
- High findings: Within 24 hours
- Questions/clarifications: Email (response within 2 business days)

---

### 7.3 Public Disclosure

**Before Mainnet Launch:**
- **No public disclosure** of specific vulnerabilities
- Responsible disclosure: wait until patches deployed

**After Mainnet Launch + Patches Deployed:**
- Publish audit report (with auditor approval)
- Blog post summarizing findings + fixes
- Security advisory if CVE-worthy vulnerabilities found

**Coordinated Disclosure Timeline:**
- Patch deployed → 30 days → Public disclosure
- Auditor and IPPAN team agree on disclosure content

---

## 8. Tracking & Metrics

### 8.1 Audit Dashboard

**Track in private GitHub project:**

| Finding ID | Severity | Status | DRI | Due Date | Fix PR | Re-Test Status | Auditor Sign-Off |
|------------|----------|--------|-----|----------|--------|----------------|------------------|
| AUD-001 | Critical | Fixed | Alice | 2025-11-26 | #123 | ✅ | ✅ |
| AUD-002 | High | In Progress | Bob | 2025-11-28 | #124 | ⏳ | ⏳ |
| AUD-003 | Medium | Open | Charlie | 2025-12-05 | - | - | - |

**Status Values:**
- `Open` - Not yet assigned/started
- `In Progress` - Fix under development
- `Fixed` - Fix merged, awaiting auditor review
- `Auditor Approved` - Auditor confirmed fix
- `Closed` - Fully resolved and signed off
- `Deferred` - Accepted as not mainnet-blocking

---

### 8.2 Metrics

**Track and report:**

- **Total findings by severity** (Critical/High/Medium/Low/Info)
- **Median time to fix** (by severity)
- **Re-test pass rate** (% of fixes that pass on first auditor review)
- **Regression count** (new issues introduced by fixes)
- **Test coverage delta** (lines added to test suite)

**Goal:**
- Critical/High findings: 100% fixed before mainnet
- Medium findings: ≥90% fixed before mainnet
- Median fix time: <24h (Critical), <72h (High), <7 days (Medium)

---

## 9. Escalation Path

### 9.1 Normal Escalation

**DRI → Lead Architect → Auditor**

If DRI cannot resolve finding within SLA:
1. DRI escalates to Lead Architect
2. Lead Architect assigns additional resources
3. If still blocked, Lead Architect contacts Auditor for clarification

---

### 9.2 Emergency Escalation

**For Critical findings or active exploitation:**

1. DRI immediately notifies Lead Architect (24/7 hotline)
2. Lead Architect mobilizes emergency response team
3. All hands on deck until critical fix deployed
4. Auditor kept in loop via real-time updates

**Emergency contacts:**
- Lead Architect: Ugo Giuliani (Signal: provided separately)
- Security Lead: [TBD based on audit firm]
- Auditor Lead: [TBD based on audit firm]

---

## 10. Post-Audit Retrospective

**After final sign-off:**

1. **Team Retrospective (Internal)**
   - What went well?
   - What could be improved?
   - Process improvements for next audit
   - Update this workflow document with lessons learned

2. **Auditor Feedback Session**
   - Quality of codebase
   - Quality of documentation
   - Responsiveness of team
   - Suggestions for future audits

3. **Public Post-Mortem** (Optional)
   - Blog post: "What we learned from our security audit"
   - Transparency builds trust with community
   - Redact specific vulnerability details (if sensitive)

---

## 11. Appendices

### Appendix A: Severity Decision Tree

```
Is there direct loss of funds? → YES → Critical
    ↓ NO
Can consensus be permanently broken? → YES → Critical
    ↓ NO
Can network halt indefinitely? → YES → Critical
    ↓ NO
Is there potential (but not guaranteed) loss of funds? → YES → High
    ↓ NO
Can network be DoS'd (but recovers)? → YES → High
    ↓ NO
Is there incorrect behavior under rare conditions? → YES → Medium
    ↓ NO
Is it a code quality / best practice issue? → YES → Low
    ↓ NO
Is it a suggestion / informational? → YES → Informational
```

### Appendix B: Example Critical Finding Response

**Finding:** Supply cap can be exceeded via integer overflow in emission calculation

**Response Timeline:**
- **Hour 0:** Finding reported by auditor
- **Hour 1:** Lead Architect confirms severity (Critical), assigns DRI
- **Hour 4:** Fix implemented (use `saturating_add` instead of `+`)
- **Hour 6:** Unit test added demonstrating overflow is prevented
- **Hour 8:** Long-run DLC gate re-run (passes with fix)
- **Hour 12:** Code review + merge to `master`
- **Hour 18:** Auditor reviews fix (requests minor change)
- **Hour 20:** Updated fix pushed
- **Hour 22:** Auditor approves fix
- **Hour 24:** Emergency release tagged, deployed to internal testnet

**Outcome:** Critical vulnerability patched in <24 hours, auditor-approved, no regressions.

---

**End of Bug Triage Workflow**

**Maintained by:** Lead Architect (Ugo Giuliani)  
**Last Updated:** 2025-11-24  
**Version:** 1.0
