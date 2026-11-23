# Post-rejection contract review plan

Baseline: `6462182`, confirmed by the maintainer as the rejected submission.
Prepared 23 November 2026. See the [cross-repository review plan](https://github.com/Mierkhat/frontend/blob/main/docs/REVIEW_PLAN.md)
for product scope, client/API fixes and the implementation sequence.

## Findings and implementation

The original guard checks exact shares, owner/recipient/domain, code/profile pins,
withdrawal and swap deltas, minimum wallet receipt, exclusive validity and replay.
The separate migration executable adds exact conversion budgets, proportional
destination deposits, destination-share floors and refunds. Both preserve donated
balances and recheck dependencies. Authorization is restricted to exact nested
transfers. Failed conditions roll back contract state; network fees and source
sequence remain outside that guarantee.

The withdrawal compiled-WASM suite already explicitly tests exclusive deadlines.
Add the equivalent migration boundary tests for timestamp equality and ledger
equality, comparing full ledger snapshots and demonstrating a valid boundary.
Rebuild unchanged pinned upstream and guard/migration WASM; run locked host tests,
formatting, artifact checksums and compatibility. Keep deployed executable hashes,
onchain identities and generated ABIs unchanged for this client/reliability pass.

Publish the concrete test additions and actual verification in a baseline-relative
change report. Existing October signed receipts and Freighter/fault tests belong
to the rejected baseline, not the new change list. Commit each verified step as
`dotmantissa <negativemantissa@gmail.com>` without co-author trailers and push at
the end of the coordinated work.

## Further development

External vault acceptance requires an exact independently operated compatible
profile and evidence of usefulness. Supporting strategies requires a separately
reviewed adapter covering unwind/queue behavior, strategy code/configuration,
fees, nested authorization, resource limits and failure-state tests. Simply
removing the current strategy rejection would weaken the supported boundary.

True TTL expiry/restoration and cold/restored headroom, further reset/fault
combinations, independent security review and a separately authorized mainnet
release remain gates. Self-issued testnet assets and a `reviewed` integration flag
do not establish yield, partnership or an audit. The hosting account is paused;
local validation is separate from production availability.
