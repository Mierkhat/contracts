# Mainnet scope and integration candidates

As of 27 October 2026, Mierkhat has no approved mainnet vault addresses and cannot sign mainnet transactions. The current six profiles target two self-issued-asset testnet vaults. They exercise selection and transaction mechanics; they do not establish support for externally operated yield vaults.

The proposed initial research shortlist is DeFindex vaults using Blend Fixed-pool USDC and XLM strategies, with USDC and XLM as possible redemption outputs where exact routes and liquidity can be verified. These are candidates, not selected vaults, partnerships or release commitments. DeFindex's [current deployment list](https://docs.defindex.io/contract-deployments) identifies strategy instances and a vault executable hash; strategy addresses must not be substituted for individual vault addresses.

The current guard rejects configured strategies. A strategy-aware adapter/profile must account for strategy identities and executable changes, withdrawal/unwind behavior, fee/report effects, nested authorization and resource limits. Removing that rejection would not establish compatibility. Each selected production vault needs an exact address, reviewed code/configuration pins, liquidity measurements, positive and rejecting tests, and independent security review before activation.

## Existing DeFindex functionality

DeFindex's [vault methods](https://docs.defindex.io/advanced-documentation/direct-contract-calls/vault-methods) and [published API](https://api.defindex.io/api-json) return underlying assets on redemption and expose per-asset minimums; the inspected withdrawal API has no desired output-token field. Manager-controlled rebalance swaps already exist in its [contract](https://github.com/defindex-io/stellar-contracts/blob/main/vault/src/lib.rs). Its [atomic vault migration](https://docs.defindex.io/integration-guide/migrating-positions/defindex-to-defindex) requires matching assets and order in the two vaults.

Mierkhat's implemented flow composes redemption and conversion with exact share spending and a final selected-token minimum. That observation does not establish ecosystem-wide novelty or buyer demand. Integrators must still identify a useful gap versus their existing contract composition, routing, authorization policies and recovery tools. A convenience feature alone is not evidence of a defensible business.

Independent review, external compatible-vault acceptance, partner pilots and a separately authorized production release remain requirements. No mainnet deployment or switch is implied by this research.
