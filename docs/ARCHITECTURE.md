# Contract architecture

The constructor pins a strategy-free DeFindex vault, ordered SAC assets, settlement token, Soroswap router/factory and direct pair executables. There are one or two assets and at most two direct swaps. An underlying already equal to settlement needs no pair. Different profiles require separate review and evidence.

`execute_exit` binds the profile/network domain, owner/recipient, exact shares, per-asset withdrawal/conversion bounds, total settlement minimum, intent identity and exclusive time/ledger deadlines. The owner authenticates the complete arguments. There is no administrator, upgrade, arbitrary call, sweep, approval or pause entry point.

The guard temporarily holds exactly the redeemed shares, receives the withdrawal, authorizes only required nested calls and sends only the new settlement delta to the owner. Executable and vault/pair identities are checked. Existing owner underlying balances and guard donations cannot make up a signed minimum shortfall.

## Atomicity and replay

A failing condition reverses all contract state, including earlier swaps. Fees and source sequence are outside that guarantee. The replay marker stays live beyond intent validity, so another envelope with the same intent cannot commit a second exit. Host tests compare touched ledger state for rollback; actual testnet evidence records independent success, minimum failure and replay rejection.

## ABI and restoration

`guard/src/types.rs` is authoritative. `AssetRoute.pair` is a bounded zero-or-one vector; the SDK's JSON transport uses a nullable pin. This avoids SDK 29 optional-UDT conversion issues. Generate client types from the CLI-built WASM, rather than editing generated types.

Archived dependencies can require a separate reviewed RestoreFootprint transaction before fresh preparation. A restore is never silently appended to an exit. Protocol-native auto-restoration can also occur. Executable checks, resource bounds and measured headroom still apply. [Release status](RELEASE_STATUS.md) distinguishes live-entry observations from pending expired-state/cold-restoration checks.
