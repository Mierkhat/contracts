# Release status

The supported testnet flow is implemented: real wallet approval, funding, trustlines, issuance, deposits, guarded exits, receipts and recovery. One pinned strategy-free profile is enabled. Mainnet is disabled. Testnet tokens have no monetary value.

The automated corpus has 10 compiled-WASM tests, 34 SDK tests, 4 provider tests, 10 actual PostgreSQL tests, 2 frontend unit tests and 12 live browser scenarios. Genuine Freighter, process-kill recovery, API outage retry, historical lookup and resource measurement have additional real network evidence. These counts describe checked-in implementation, not a claim that remote CI already passed.

## Remaining gates

- Independent review of the composed contracts, authorization and client policies.
- Genuine external partner acceptance and an independently operated compatible profile.
- Archived-to-live restoration after TTL expiry and cold/restored resource headroom.
- Wallet identity switching during an unresolved extension prompt.
- Extended post-admission/provider faults and network-reset recovery beyond tested epoch rejection.
- A separate mainnet decision, deployment review and authorization.

`reviewed` means source/profile integration verification, not an independent audit, partnership or mainnet readiness. Incident handling can stop new construction at the service boundary; it cannot upgrade or administratively pause the immutable guard. Continue public-hash reconciliation during an incident.
