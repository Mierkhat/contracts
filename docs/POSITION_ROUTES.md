# Testnet vaults and redemption outputs

The deployment now has two actual strategy-free DeFindex vaults: the original MKA/MKB vault and an MKOUT vault. Each supports MKOUT, MKA and MKB redemption outputs. A separate immutable guard profile binds every vault/output combination to the exact vault, router, factory, settlement asset and required direct pairs. The original guard and profile remain valid; guard WASM and ABI are unchanged.

`evidence/testnet-position-routes.json` records the actual MKA/MKB liquidity transaction, second vault deployment/seed and five additional guard deployments. Bundle fields were verified against actual testnet instances, executable hashes, metadata and on-chain profiles before both application registries were updated. The original two MKA/MKOUT and MKB/MKOUT pairs remain in use.

An output equal to an underlying asset is returned directly; the other withdrawn assets are converted into it. The single-asset MKOUT vault can return MKOUT without a swap. Neither case may spend the owner's existing token balances to satisfy the final minimum. Positions are share balances grouped by vault, not by deposit or output profile.

Reproduce deployment from the combined workspace with `./scripts/pnpm --filter @mierkhat/sdk exec tsx scripts/testnet-position-routes.ts`. It needs the existing dedicated testnet fixture keys in the ignored local source and actual token balances. It checkpoints public transaction hashes before submission and retains exact signed retry bytes only in the ignored private directory. An uncertain operation is reconciled using the same hash and bytes.

These self-issued testnet assets have no monetary value. The deployment does not establish support for strategy-bearing production vaults or authorize mainnet.
