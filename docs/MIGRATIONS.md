# Immutable partial vault migration

`migration/` is a separate executable from `guard/`. The original withdrawal artifact is unchanged. The migration WASM SHA-256 is `849e722114e5d48c3f4768590e3d6636045da8f5d3877aaafe71add36f834f6c`; both enabled testnet migration profiles use it. Their addresses, exact source/destination pins and deployment transactions are in [testnet-migration-routes.json](../evidence/testnet-migration-routes.json) and the backend SDK registry.

## Profile and intent

A profile fixes the source withdrawal route, destination vault and up to two destination assets/direct conversion pairs. Source and destination must differ. Both vaults must have the exact configured strategy-free asset sets. Assets are SACs and every external executable/pair identity is checked before and after execution. The current code does not approve strategy-bearing or queued vaults.

The migration intent wraps the existing source-share and withdrawal/conversion conditions. It adds exact intermediary-token conversion inputs, positive per-asset deposit minima and a positive minimum destination-share receipt. Network, guard, profile, owner/recipient, expiry and intent ID are bound into authorization and replay protection. Only the owner receives the result.

## Execution and invariants

1. Snapshot owner/guard balances, including both share tokens and every tracked underlying/intermediary/destination asset. Reserve the bounded replay marker.
2. Collect exactly the authorized source shares. Withdraw and convert the actual newly released assets into the intermediary token using the existing source route checks.
3. Spend only the reviewed conversion inputs, never pre-existing guard donations. Check actual input/output deltas against router results and deposit floors.
4. Derive exact deposit transfers using the pinned strategy-free vault's proportional arithmetic, including upward rounding for non-enforced assets. Authorize only these exact nested transfers. Execute the deposit and verify actual asset consumption.
5. Measure newly minted destination shares, require the signed destination minimum and transfer only that delta to the owner.
6. Refund every residual tracked token. Require all guard baselines restored, exact source-share spending, exact destination-share delivery and owner token deltas equal to the recorded refunds. Recheck dependencies and emit the migration receipt.

There are no approvals, mutable registries, upgrades, pause, fee collection or sweeping methods. Failed conditions propagate to the root invocation and revert action state. Network fees/source sequence can still be consumed. A migration preserves the source remainder; it does not create another cosmetic deposit position.

## ABI and receipts

The root is `execute_migration(MigrationIntent)`, with an independently generated SDK specification. The profile hash encodes `MigrationProfile`; it must not be replaced with the source `RouteProfile` hash. Receipt fields include the source share spend, internal settlement quantity, destination identity/shares, deposits, conversions, ordered refund assets/amounts, intent/profile hashes and ledger/time.

The internal settlement quantity is not the wallet payout. Consumers must show destination shares and refunds and verify the receipt against the actual root method, profile and signed intent.

## Verification

`./scripts/build-guard.sh` builds both executables with the pinned CLI and lock. `cargo test --workspace --locked` runs the original thirteen withdrawal cases and five migration cases against actual compiled upstream WASM with strict execution authorization. Migration cases exercise one/two-asset destinations, two conversion legs, partial source spending, donated balances, refunds, replay, destination-floor full-state rollback, absent authority, expiry, excessive conversion input, changed destination code/strategies, malformed vectors and wrong recipient.

Artifact verification reproduces the original withdrawal hash and both migration profile pins. The frontend's genuine Freighter acceptance records both migration directions and exact observed source/destination/refund balance deltas. The separate signed testnet rollback helper verifies failure at the destination minimum and compares all non-account footprint values. These engineering results do not constitute an independent security audit or mainnet approval.
