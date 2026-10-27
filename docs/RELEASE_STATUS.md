# Release status

The supported testnet flow is implemented: real wallet approval, funding, trustlines, issuance, deposits, guarded exits, receipts and recovery. One pinned strategy-free profile is enabled. Mainnet is disabled. Testnet tokens have no monetary value.

The automated corpus has 13 compiled-WASM tests, 34 SDK tests, 4 provider tests, 10 actual PostgreSQL tests, 2 frontend unit tests and 14 live browser scenarios. Genuine Freighter including an actual account switch during a pending prompt, process-kill recovery, API/RPC response loss, independent concurrent providers, historical lookup and resource measurement have additional real network evidence. These counts describe checked-in implementation, not a claim that remote CI already passed.

## Remaining gates

- Independent review of the composed contracts, authorization and client policies.
- Genuine external partner acceptance and an independently operated compatible profile.
- Archived-to-live restoration after TTL expiry and cold/restored resource headroom.
- Actual full-network-reset recovery beyond tested epoch rejection. The four recorded response-loss/concurrency/process boundaries pass; further fault combinations remain review work.
- A separate mainnet decision, deployment review and authorization.

`reviewed` means source/profile integration verification, not an independent audit, partnership or mainnet readiness. Incident handling can stop new construction at the service boundary; it cannot upgrade or administratively pause the immutable guard. Continue public-hash reconciliation during an incident.
