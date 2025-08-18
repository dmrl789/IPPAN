# 🌐 **IPPAN Domain & Handle Fees — 20 Year Sliding Scale**

## Overview

IPPAN implements a **predictable, immutable 20-year sliding scale fee system** that discourages domain squatting while rewarding long-term holders. This system ensures domain accessibility while maintaining network sustainability.

## 1. Core Rules (Immutable, Hard-coded)

### Premium Multipliers
- **Standard domains (.ipn)**: ×1 multiplier
- **IoT domains (.iot)**: ×2 multiplier  
- **Premium domains (.ai, .m)**: ×10 multiplier

### Fee Schedule
- **Year 1**: 0.20 IPN × premium multiplier
- **Year 2**: 0.02 IPN × premium multiplier
- **Year 3-11**: 0.01 IPN decreasing by 0.001 each year × multiplier
- **Year 12+**: 0.001 IPN (floor) × multiplier

## 2. 20-Year Fee Schedule (Standard .ipn domains)

| Year | Fee (IPN) | Cumulative | Year | Fee (IPN) | Cumulative |
|------|-----------|------------|------|-----------|------------|
| 1    | 0.200     | 0.200      | 11   | 0.001     | 0.265      |
| 2    | 0.020     | 0.220      | 12   | 0.001     | 0.266      |
| 3    | 0.009     | 0.229      | 13   | 0.001     | 0.267      |
| 4    | 0.008     | 0.237      | 14   | 0.001     | 0.268      |
| 5    | 0.007     | 0.244      | 15   | 0.001     | 0.269      |
| 6    | 0.006     | 0.250      | 16   | 0.001     | 0.270      |
| 7    | 0.005     | 0.255      | 17   | 0.001     | 0.271      |
| 8    | 0.004     | 0.259      | 18   | 0.001     | 0.272      |
| 9    | 0.003     | 0.262      | 19   | 0.001     | 0.273      |
| 10   | 0.002     | 0.264      | 20   | 0.001     | 0.274      |

**Total cost for 20 years: 0.274 IPN**

## 3. Premium Domain Examples

### .ai Domain (×10 multiplier)
- **Year 1**: 2.0 IPN
- **Year 2**: 0.20 IPN  
- **Year 11+**: 0.01 IPN (floor)
- **Total 20 years**: 2.74 IPN

### .iot Domain (×2 multiplier)
- **Year 1**: 0.40 IPN
- **Year 2**: 0.04 IPN
- **Year 11+**: 0.002 IPN (floor)
- **Total 20 years**: 0.548 IPN

## 4. Rationale

### Year 1 Barrier
- **Purpose**: Prevents mass domain squatting
- **Cost**: 0.20 IPN (affordable but meaningful)
- **Effect**: Encourages thoughtful domain selection

### Year 2 Transition
- **Purpose**: Fair pricing for active users
- **Cost**: 0.02 IPN (10x reduction from Year 1)
- **Effect**: Rewards continued engagement

### Years 3-11 Tapering
- **Purpose**: Graceful cost reduction for long-term holders
- **Pattern**: Linear decrease from 0.009 to 0.001 IPN
- **Effect**: Incentivizes long-term domain ownership

### Year 12+ Floor
- **Purpose**: Guaranteed perpetual renewal
- **Cost**: 0.001 IPN (dust-level fee)
- **Effect**: Domains never become unaffordable

## 5. Implementation

### Rust Function
```rust
/// Returns the yearly domain fee in micro-IPN (1e-6 IPN units).
pub fn domain_fee(year: u32, premium_mult: u32) -> u64 {
    let base: f64 = if year == 1 {
        0.20
    } else if year == 2 {
        0.02
    } else {
        let decayed = 0.01 - 0.001 * (year as f64 - 3.0);
        if decayed < 0.001 { 0.001 } else { decayed }
    };
    (base * premium_mult as f64 * 1_000_000.0).round() as u64
}
```

### CLI Usage
```bash
# Register example.ipn for 1 year
ippan domain register example.ipn --years 1
# Fee: 0.200000 IPN

# Register example.ai for 1 year  
ippan domain register example.ai --years 1
# Fee: 2.000000 IPN (×10 premium multiplier)

# Renew example.ipn after 5 years (Year 6)
ippan domain renew example.ipn --years 1
# Fee: 0.006000 IPN
```

## 6. Sustainability Analysis

### Revenue Projections
- **Standard domain (20 years)**: 0.274 IPN total
- **Premium domain (20 years)**: 2.74 IPN total
- **IoT domain (20 years)**: 0.548 IPN total

### Network Impact
- **Barrier to squatting**: Year 1 fee prevents mass registration
- **Long-term affordability**: Floor ensures perpetual renewal
- **Premium funding**: Premium domains fund ecosystem development
- **Predictable costs**: Users can plan long-term domain costs

## 7. Migration from Old System

### Old vs New Fees
| Domain Type | Old Fee (1 year) | New Fee (Year 1) | Change |
|-------------|------------------|------------------|---------|
| Standard | 8.00 IPN | 0.20 IPN | -97.5% |
| Premium | 10.00 IPN | 2.00 IPN | -80.0% |
| IoT | 5.00 IPN | 0.40 IPN | -92.0% |

### Benefits
- **Massive cost reduction** for new registrations
- **Predictable long-term costs**
- **Fair premium pricing**
- **Anti-squatting protection**

## 8. Technical Details

### Fee Calculation Units
- **Internal**: Micro-IPN (1e-6 IPN)
- **Display**: IPN with 6 decimal places
- **Precision**: 64-bit integers for calculations

### Premium Multiplier Detection
```rust
pub fn from_domain(domain: &str) -> PremiumMultiplier {
    if domain.ends_with(".ai") || domain.ends_with(".m") {
        PremiumMultiplier::Premium
    } else if domain.ends_with(".iot") {
        PremiumMultiplier::IoT
    } else {
        PremiumMultiplier::Standard
    }
}
```

### Validation Rules
- **Year range**: 1-20 years for registration
- **Domain format**: Must end with supported TLD
- **Fee calculation**: Automatic based on domain type and years
- **Payment**: All fees flow to Global Fund

## 9. Future Considerations

### Potential Extensions
- **Additional TLDs**: New premium multipliers for specialized domains
- **Dynamic pricing**: Market-based adjustments (governance-controlled)
- **Bulk discounts**: Reduced fees for multi-year registrations
- **Early renewal bonuses**: Incentives for proactive renewal

### Governance Controls
- **Fee schedule**: Immutable once set
- **Premium multipliers**: Adjustable via governance vote
- **New TLDs**: Addable via governance proposal
- **Floor adjustments**: Emergency changes only

---

✅ **This fee system is immutable, predictable, human-fair, and sustainable.**
It discourages squatting, rewards long-term holders, and guarantees perpetual renewal at dust-level cost.
