# Mierkhat contracts

Immutable Soroban guard for a pinned strategy-free DeFindex vault and direct Soroswap conversions. No administrator, upgrade entry point, arbitrary call, approval or sweep method. The owner is the recipient. The guard preserves pre-existing owner assets and guard donations, checks exact shares and total received, and fails atomically when a signed condition is not met.

Guard: Rust1.93.0, SDK29.0.1, Stellar CLI28.1.0. The CLI rejects the originally proposed Rust1.91.0. SDK29 requires CLI specification shaking, so build the guard through Stellar CLI, not by bypassing its build-system check.

```sh
# At the project root (or this directory in an extracted repository)
python3 contracts/scripts/build-upstream.py
cd contracts
./scripts/build-guard.sh
cargo test --workspace --locked
```

For a standalone contracts checkout, omit the leading `contracts/` in the first command. Upstream scripts require Rust1.91.0 plus the wasm32v1-none target, in addition to the guard's pinned toolchain. They clone only into ignored `upstream/`, build unchanged pinned repositories under original lockfiles, and save artifact provenance in `evidence/upstream-build.json`. Historical contractimport filenames ending in optimized.wasm are exact byte copies, not optimizer claims.

`AssetRoute.pair` is a bounded zero-or-one vector on chain; JSON SDK transport uses null or a CodePin. This corrects SDK29's native Option<UDT> conversion compilation failure. Generated ABI is authoritative. Zero entries are permitted only for an underlying equal to settlement.

Ten real compiled-WASM tests cover successful one/two-asset exits, settlement underlying, exact nested contract authorization, replay/TTL, deadline/vector/domain mutations, missing root/child-only authorization, donation preservation and donation-funded-minimum rejection, second-swap/final-total failure rollback over every ledger entry, runtime code changes and Rust/XDR vectors. Broad mocks are confined to seeding balances, liquidity and initial deposits; guard execution uses explicit owner trees.

`evidence/testnet-fixture.json` contains a real protocol29 signed success and a ledger-confirmed failure of the final minimum with matching before/after hashes for all existing footprint entries. Only public data is committed. The dedicated testnet keys and retry envelopes are local ignored files. The reproducible harness lives in `backend/sdk/scripts/testnet-fixture.ts` in the combined checkout because it exercises the public SDK; it creates only named test assets and never uses a user's wallet funds.

These are engineering proofs, not an independent security review or an external integrator acceptance. The testnet profile remains disabled in public application registries. CLI28.1.0 is the current stable release but warns that protocol29 settings may be incomplete; complete resource-limit review and 20% worst-case headroom remain a release gate. Network fees and source sequence can be consumed by failed transactions.
