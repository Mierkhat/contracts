# Contributing to Mierkhat

1. Choose an issue and request assignment before starting. You must be assigned every real Mierkhat issue referenced in your PR title, description, or linked issues, including sibling repositories.
2. Fork the repository, fetch upstream `dev`, and create a feature branch from `dev`. Keep each PR focused on its assigned issue.
3. Implement the change and run the local checks below. Install Rust 1.91.0/1.93.0 and Stellar CLI 28.1.0 as pinned in `.github/workflows/ci.yml`; the tests build the contract WASM first.
4. Open the PR against **`dev` only** and complete the PR template. Describe the change, why it is needed, assigned issue links such as `Closes #42`, and the commands and results used to verify it.
5. Fix failing checks, resolve review feedback, and update your branch when `dev` changes. The bot merges eligible PRs after standard CI, security, and description checks pass for the current revision.

```sh
cargo +1.93.0 fmt --all --check
python3 scripts/build-upstream.py
./scripts/build-guard.sh
cargo +1.93.0 test --workspace --locked
```

CI also scans secrets, checks workflow security, and tests the contributor automation. CI or automation changes require maintainer review. Deeper compatibility, browser/evidence, and release checks remain on `main`.

Contributor PRs targeting another branch, missing an assigned issue, or referencing an issue you are not assigned to are automatically closed. Request assignment, correct the target or links, and reopen the PR. API outages preserve PRs for a retry.

After a verified merge into `dev`, the bot closes referenced frontend/backend/contracts issues as `completed` only when all PR checks pass and every real referenced issue is still assigned to the original contributor. Any unassigned issue blocks all remaining closures. Failed closures are retried; assignees remain unchanged.

The bot uses merge commits to preserve original contributor authors and commits. It does not rewrite attribution or issue assignees, add co-author trailers, or auto-merge changes to its own CI and security policy. Assignment enforcement mirrors Hyperion's repository-role and authenticated helper-bot exemptions; auto-merge and issue completion still require every referenced issue to be assigned.
