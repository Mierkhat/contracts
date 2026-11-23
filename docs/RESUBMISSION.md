# Contract changes since rejection

Rejected baseline: `6462182`, confirmed by the maintainer. Verified locally
24 November 2026. `86f3d89` committed the review plan before implementation.
`f9d86ef` added two compiled-WASM migration tests:

- Equality at the time deadline or ledger deadline rejects with contract error
  1003 and an unchanged full ledger snapshot, for one-asset and two-asset
  destination layouts.
- Both layouts succeed at the last valid time and ledger, spending exactly the
  selected source shares and crediting the receipt's destination shares.

The locked fixture suite now passes 20 tests (rejected baseline: 18).
Pinned upstream builds, both guard builds, locked workspace tests, formatting
and artifact verification passed. The rebuilt withdrawal and migration WASM
hashes match deployed proof: `515bcffd6ee75c51cf4ff65b054751e562a5f6ad8814cce4c9b9572942e91269`
and `849e722114e5d48c3f4768590e3d6636045da8f5d3877aaafe71add36f834f6c`.
Existing event-publish/deployer deprecation warnings remain; changing deployed
executables is outside this release. History secret scans and workflow checks
passed. Final documentation/compatibility commits follow the test change.

No contract executable, ABI, profile, token or deployed registry was changed.
SDK restoration now covers migration destination dependencies in the backend;
frontend recovery/form changes are documented in the
[cross-repository report](https://github.com/Mierkhat/frontend/blob/main/docs/RESUBMISSION.md).

October Freighter receipts, fault experiments and public deployment evidence
were present in the rejected version. This update claims new boundary tests,
not new on-chain deployments or fresh signed-wallet acceptance. Vercel remains
paused; local verification succeeded. Strategy-free/self-issued testnet fixtures
do not establish external yield strategy support or independent security review.

The [review plan](REVIEW_PLAN.md) and
[whole-project development scope](https://github.com/Mierkhat/frontend/blob/main/docs/REVIEW_PLAN.md)
retain cold restoration, strategy-aware adapter, external useful-vault pilot,
service operability and independent/mainnet review as separate gates.
