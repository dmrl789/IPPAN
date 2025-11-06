# IPPAN Security

## Overview
- Provides runtime security primitives for IPPAN nodes and APIs.
- Coordinates rate limiting, circuit breaking, input validation, and audit logging.
- Helps operators enforce deterministic safety and produce evidence for governance reviews.

## Key Modules
- `rate_limiter`: async rate limiting with per-endpoint statistics.
- `circuit_breaker`: guards downstream services with configurable failure thresholds.
- `validation`: reusable input validation rules for RPC and API payloads.
- `audit`: asynchronous audit logging for security events and access decisions.

## Integration Notes
- Initialize `SecurityManager` with deployment-specific `SecurityConfig` values.
- Call `check_request`, `record_success`, and `record_failure` inside network handlers.
- Inspect `SecurityStats` when wiring observability or incident response tooling.
