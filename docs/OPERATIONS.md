# Builds and deployment

Use Rust 1.93.0 for the guard, Rust 1.91.0 for unchanged historical upstream sources, Soroban SDK 29.0.1 and Stellar CLI 28.1.0. Toolchain files and `evidence/toolchain.json` record the pins.

For Linux x64:

```sh
curl -fsSL https://github.com/stellar/stellar-cli/releases/download/v28.1.0/stellar-cli-28.1.0-x86_64-unknown-linux-gnu.tar.gz -o /tmp/mierkhat-stellar-cli.tar.gz
echo 'c1680deee94301d33ada7a17f98411e642a4248c727afbd2e43050d345746462  /tmp/mierkhat-stellar-cli.tar.gz' | sha256sum --check
mkdir -p "$HOME/.local/bin"
tar -xzf /tmp/mierkhat-stellar-cli.tar.gz -C "$HOME/.local/bin"
export PATH="$HOME/.local/bin:$PATH"
stellar --version
```

## Upstream provenance

`scripts/build-upstream.py` uses Soroswap commit `6eade008ee6794075ad941609e0fef0d1124d3eb` and DeFindex commit `a0fcabfae38f1c7b0c9c2f10ff51d225f8edc2f1`. It rejects dirty source, builds with original locks and writes `evidence/upstream-build.json`. Historical `.optimized.wasm` imports are exact copies, not optimizer claims. Source/build output remains ignored.

`./scripts/build-guard.sh` validates the CLI and performs required specification shaking. `scripts/verify-artifacts.py` compares upstream bytes with provenance and guard bytes with the deployed proof.

## Deployment and ABI

A new deployment requires a reviewed profile, network/epoch identity and consistent client pins; an API-discovered address alone cannot authorize it. The current release has no authorized mainnet deployment.

The actual fixture harness lives in the [backend SDK](https://github.com/Mierkhat/backend/blob/main/sdk/scripts/testnet-fixture.ts). It expects sibling checkouts, built WASM and dedicated testnet accounts. Generated private keys remain outside source in ignored `.tooling/testnet/`. Ordinary contract builds do not require it.

After a contract change, build the guard and regenerate SDK bindings with an absolute WASM path. Coordinate deployment pins and frontend SDK archive updates, then repeat host and real testnet acceptance. Successful compilation is not deployment evidence.
