# Mierkhat contracts

[![Verify](https://github.com/Mierkhat/contracts/actions/workflows/verify.yml/badge.svg)](https://github.com/Mierkhat/contracts/actions/workflows/verify.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Mierkhat's immutable Soroban guard redeems an exact number of DeFindex vault shares, converts withdrawn assets through direct Soroswap routes and sends one settlement token to the owner. The signed intent sets a minimum total receipt, individual bounds and validity deadlines. A failed condition rolls back contract changes atomically. Network fees and source sequence can still be consumed.

The guard has no administrator, upgrade, sweep or pause entry point. It preserves the owner's existing underlying balances and tokens already held by the guard. The current profile supports a strategy-free vault with one or two assets and at most two direct swaps.

Migrations use a separate immutable executable in `migration/`. It moves a chosen portion between the two different testnet asset sets, enforces minimum destination shares and returns residual tokens to the owner. The original withdrawal executable is unchanged. See [migration invariants and evidence](docs/MIGRATIONS.md).

## Stack

| Component              | Technology                                        |
| ---------------------- | ------------------------------------------------- |
| Guard                  | Rust 1.93.0, Soroban SDK 29.0.1                   |
| Contract build         | Stellar CLI 28.1.0, `wasm32v1-none`               |
| Pinned upstream builds | Rust 1.91.0, DeFindex and Soroswap original locks |
| Verification           | Compiled-WASM host tests, public testnet receipts |
| Build tooling          | Python 3, Bash, Git, Cargo                        |

## Build and test

Install rustup, Python 3, Git and the pinned Stellar CLI. Its installation and archive checksum are in [operations](docs/OPERATIONS.md).

```sh
git clone https://github.com/Mierkhat/contracts.git
cd contracts
rustup toolchain install 1.91.0 --profile minimal --target wasm32v1-none
rustup toolchain install 1.93.0 --profile minimal --component clippy --component rustfmt --target wasm32v1-none
python3 scripts/build-upstream.py
./scripts/build-guard.sh
cargo test --workspace --locked
cargo fmt --all --check
python3 scripts/verify-artifacts.py
```

The upstream builder uses exact commits/original locks, rejects modified source and records SHA-256 provenance. Builds stay in ignored `upstream/`, `artifacts/` and `target/`. Build the guard through Stellar CLI: SDK 29 requires specification shaking and this CLI requires Rust 1.93.0. Direct Cargo WASM output is not an equivalent replacement.

Eighteen compiled-WASM tests cover one/two-asset exits, settlement as an underlying asset, exact nested authorization, replay/deadlines, intent mutations, donation preservation, donation-funded minimum rejection, whole-state rollback, executable changes and Rust/XDR vectors. Seeding fixtures may use test authorization helpers; guard execution uses explicit owner trees.

## Repository layout

```text
guard/        Original immutable withdrawal guard and signed types
migration/    Separate immutable partial vault migration guard
fixtures/     Real compiled-WASM integration tests
scripts/      Pinned builds and artifact verification
evidence/     Public receipts, provenance and network observations
docs/         Invariants, deployment instructions and release status
Cargo.lock    Locked Rust dependencies
```

## Testnet deployment

The enabled application profile uses this guard:

```text
CA42L7LDVJMI2BXBNRURXF6K6MVCTBZGND6RP5NYFEGIGWR3HTLNTT74
```

Its WASM SHA-256 is `515bcffd6ee75c51cf4ff65b054751e562a5f6ad8814cce4c9b9572942e91269`. Complete pins are in the [backend SDK registry](https://github.com/Mierkhat/backend/blob/main/sdk/deployments/index.json) and independently bundled in the frontend.

[Public evidence](docs/EVIDENCE.md) records actual exits, rollback, replay rejection, Freighter approvals, crash recovery, historical lookup and resource headroom. These are engineering checks, not an independent audit. Mainnet is disabled; [release status](docs/RELEASE_STATUS.md) records remaining gates. Testnet assets have no monetary value. Resets and entry expiry can affect historical availability.

## Documentation

- [Partial cross-asset migration](docs/MIGRATIONS.md)
- [Architecture and invariants](docs/ARCHITECTURE.md)
- [Builds, deployment and ABI generation](docs/OPERATIONS.md)
- [Evidence index](docs/EVIDENCE.md) and [release status](docs/RELEASE_STATUS.md)
- [Testnet position routes](docs/POSITION_ROUTES.md)
- [Mainnet candidates and existing DeFindex capabilities](docs/MAINNET_SCOPE.md)
- [Backend and SDK](https://github.com/Mierkhat/backend) and [frontend](https://github.com/Mierkhat/frontend)

## Combined workspace

In the original combined workspace, run `python3 contracts/scripts/build-upstream.py` from its root, then the remaining commands inside `contracts/`. Both layouts use the same source and locks.

## Contributing and security

Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request. Report sensitive issues through the [security policy](SECURITY.md).

## License

Mierkhat-authored code uses the [MIT license](LICENSE). Dependencies, upstream protocols, fonts where bundled and brand usage retain the terms described in [third-party notices](THIRD_PARTY_NOTICES.md).
