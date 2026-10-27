# Public testnet evidence

These files contain public observations, never private keys or exact retry envelopes. They were checked against actual Stellar testnet. Later resets/entry expiry can change current availability without changing a historical observation.

| Evidence | Observation |
| --- | --- |
| [testnet-position-routes.json](../evidence/testnet-position-routes.json) | Actual second vault, direct-pair liquidity and immutable output-profile deployments |
| [freighter-position-routes.json](../evidence/freighter-position-routes.json) | Two actual vault positions, all six output routes, genuine Freighter approvals, exact share/token balances, receipt recovery and six responsive/contrast captures |
| [testnet-fixture.json](../evidence/testnet-fixture.json) | Signed success, minimum error 1012 with touched-state rollback, replay error 1008 and marker TTL |
| [testnet-journey.json](../evidence/testnet-journey.json) | Funding, trustlines, issuer payments, deposit, guarded exit and balance invariants |
| [freighter-identity-change.json](../evidence/freighter-identity-change.json) | Actual account switch during a pending signing prompt, late original approval discarded, zero submissions and connected keyboard tabs |
| [freighter-journey.json](../evidence/freighter-journey.json) | Real extension approvals, receipt, separate restore and API outage retry |
| [testnet-recovery-matrix.json](../evidence/testnet-recovery-matrix.json) | Four actual guarded-exit response-loss, provider-concurrency and post-send process-kill boundaries with full balance/digest checks |
| [restoration-readiness.json](../evidence/restoration-readiness.json) | Actual fixed-entry TTLs and earliest expiry; explicitly waiting while entries remain live |
| [testnet-crash-recovery.json](../evidence/testnet-crash-recovery.json) | Durable admission, process kill, same-hash recovery and confirmation |
| [archive-observation.json](../evidence/archive-observation.json) | Horizon recovery outside RPC retention |
| [testnet-resource-headroom.json](../evidence/testnet-resource-headroom.json) | Actual two-asset/two-swap metrics against network limits |
| [upstream-build.json](../evidence/upstream-build.json) | Source commits, locks, sizes and SHA-256 provenance |
| [toolchain.json](../evidence/toolchain.json) | Tool pins and deployed guard hash |

The Freighter exit hash is `295cc18416587e0551bb8f98630f593b3c13a4522c1208297ac1ff3559de6c9e`.

The restore confirmed while target entries were live; it does not prove an archived-to-live transition. Horizon lacks contract return metadata, so transaction success alone is not a decoded guard receipt. Positive tests do not replace independent security review. See [remaining gates](RELEASE_STATUS.md).
