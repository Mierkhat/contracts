use super::*;
use mierkhat_migration::{MigrationClient, MigrationIntent, MigrationProfile};
use soroban_sdk::{vec, TryFromVal, Val};
const MIGRATION: &[u8] =
    include_bytes!("../../../target/wasm32v1-none/release/mierkhat_migration.wasm");

fn setup(multi_destination: bool) -> (Fixture, Address, Address, MigrationIntent) {
    let f = fixture(false, multi_destination);
    let e = &f.env;
    e.mock_all_auths();
    let admin = Address::generate(e);
    let extra = e
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(e, &extra).mint(&f.owner, &1_000_000_000_000);
    router::Client::new(e, &f.profile.router.address).add_liquidity(
        &f.settlement,
        &extra,
        &100_000_000_000,
        &100_000_000_000,
        &1,
        &1,
        &f.owner,
        &2000,
    );
    let assets = if multi_destination {
        vec![e, f.assets.get(0).unwrap(), extra]
    } else {
        vec![e, f.settlement.clone()]
    };
    let configs = assets
        .iter()
        .map(|a| vault::AssetStrategySet {
            address: a,
            strategies: vec![e],
        })
        .collect::<std::vec::Vec<_>>();
    let destination = e.register(
        vault::WASM,
        (
            Vec::from_slice(e, &configs),
            Map::from_array(
                e,
                [
                    (0u32, admin.clone()),
                    (1, admin.clone()),
                    (2, admin.clone()),
                    (3, admin.clone()),
                ],
            ),
            0u32,
            admin,
            0u32,
            f.profile.router.address.clone(),
            Map::from_array(
                e,
                [
                    (
                        String::from_str(e, "name"),
                        String::from_str(e, "Destination"),
                    ),
                    (String::from_str(e, "symbol"), String::from_str(e, "DST")),
                ],
            ),
            false,
        ),
    );
    let amounts = assets
        .iter()
        .map(|_| 10_000_000_000i128)
        .collect::<std::vec::Vec<_>>();
    vault::Client::new(e, &destination).deposit(
        &Vec::from_slice(e, &amounts),
        &Vec::from_slice(e, &amounts),
        &f.owner,
        &false,
    );
    let routes = assets
        .iter()
        .map(|a| mierkhat_migration::AssetRoute {
            asset: a.clone(),
            pair: if a == f.settlement {
                vec![e]
            } else {
                let pair_address = router::Client::new(e, &f.profile.router.address)
                    .router_pair_for(&a, &f.settlement);
                let p = pin(e, &pair_address, pair::WASM);
                vec![
                    e,
                    mierkhat_migration::CodePin {
                        address: p.address,
                        wasm_hash: p.wasm_hash,
                    },
                ]
            },
        })
        .collect::<std::vec::Vec<_>>();
    // Convert identical ABI structures through XDR values, without linking either executable.
    let value: Val = f.profile.clone().into_val(e);
    let source = mierkhat_migration::RouteProfile::try_from_val(e, &value).unwrap();
    let dest_pin = pin(e, &destination, vault::WASM);
    let profile = MigrationProfile {
        source,
        destination: mierkhat_migration::CodePin {
            address: dest_pin.address,
            wasm_hash: dest_pin.wasm_hash,
        },
        destination_assets: Vec::from_slice(e, &routes),
    };
    let guard = e.register(MIGRATION, (profile,));
    let value: Val = f.intent.clone().into_val(e);
    let mut exit = mierkhat_migration::ExitIntent::try_from_val(e, &value).unwrap();
    exit.guard = guard.clone();
    exit.profile_hash = MigrationClient::new(e, &guard).profile_hash();
    let draft = MigrationIntent {
        exit,
        convert_inputs: if multi_destination {
            vec![e, 400_000_000, 400_000_000]
        } else {
            vec![e, 0]
        },
        min_deposit: if multi_destination {
            vec![e, 1, 1]
        } else {
            vec![e, 1]
        },
        min_destination_shares: 1,
    };
    e.mock_auths(&[]);
    (f, destination, guard, draft)
}
fn auth(f: &Fixture, guard: &Address, draft: &MigrationIntent) {
    f.env.mock_auths(&[MockAuth {
        address: &f.owner,
        invoke: &MockAuthInvoke {
            contract: guard,
            fn_name: "execute_migration",
            args: (draft.clone(),).into_val(&f.env),
            sub_invokes: &[MockAuthInvoke {
                contract: &f.vault,
                fn_name: "transfer",
                args: (f.owner.clone(), guard.clone(), draft.exit.shares).into_val(&f.env),
                sub_invokes: &[],
            }],
        },
    }]);
}
#[test]
fn partial_migrations_preserve_remainders_and_refund_every_asset() {
    for multi in [false, true] {
        let (f, destination, guard, draft) = setup(multi);
        let e = &f.env;
        let before = token::Client::new(e, &f.vault).balance(&f.owner);
        let dest_before = token::Client::new(e, &destination).balance(&f.owner);
        // Donations include source and destination shares as well as tracked SAC assets.
        let tokens = vec![
            e,
            f.assets.get(0).unwrap(),
            f.settlement.clone(),
            f.vault.clone(),
            destination.clone(),
        ];
        for token_address in tokens.iter() {
            token::Client::new(e, &token_address)
                .mock_auths(&[MockAuth {
                    address: &f.owner,
                    invoke: &MockAuthInvoke {
                        contract: &token_address,
                        fn_name: "transfer",
                        args: (f.owner.clone(), guard.clone(), 12345i128).into_val(e),
                        sub_invokes: &[],
                    },
                }])
                .transfer(&f.owner, &guard, &12345);
        }
        auth(&f, &guard, &draft);
        let receipt = MigrationClient::new(e, &guard).execute_migration(&draft);
        assert_eq!(
            token::Client::new(e, &f.vault).balance(&f.owner),
            before - 12345 - draft.exit.shares
        );
        assert_eq!(
            token::Client::new(e, &destination).balance(&f.owner),
            dest_before - 12345 + receipt.destination_shares
        );
        assert!(receipt.destination_shares > 0 && before - draft.exit.shares > 0);
        for token_address in tokens.iter() {
            assert_eq!(token::Client::new(e, &token_address).balance(&guard), 12345);
        }
        assert!(MigrationClient::new(e, &guard)
            .try_execute_migration(&draft)
            .is_err());
    }
}
#[test]
fn destination_minimum_failure_reverts_every_ledger_entry() {
    let (f, _, guard, mut draft) = setup(true);
    draft.min_destination_shares = 100_000_000_000_000;
    auth(&f, &guard, &draft);
    let before = f.env.to_snapshot().ledger;
    assert_eq!(
        MigrationClient::new(&f.env, &guard).try_execute_migration(&draft),
        Err(Ok(soroban_sdk::Error::from_contract_error(1012)))
    );
    assert_eq!(before, f.env.to_snapshot().ledger);
    draft.min_destination_shares = 1;
    auth(&f, &guard, &draft);
    MigrationClient::new(&f.env, &guard).execute_migration(&draft);
}
#[test]
fn missing_auth_expiry_and_overspending_are_rejected() {
    let (f, _, guard, draft) = setup(true);
    let before = f.env.to_snapshot().ledger;
    assert!(MigrationClient::new(&f.env, &guard)
        .try_execute_migration(&draft)
        .is_err());
    assert_eq!(before, f.env.to_snapshot().ledger);
    let mut invalid = draft.clone();
    invalid.exit.valid_before_time = 1000;
    auth(&f, &guard, &invalid);
    let before = f.env.to_snapshot().ledger;
    assert!(MigrationClient::new(&f.env, &guard)
        .try_execute_migration(&invalid)
        .is_err());
    assert_eq!(before, f.env.to_snapshot().ledger);
    invalid = draft.clone();
    invalid.convert_inputs = vec![&f.env, i128::MAX, 0];
    auth(&f, &guard, &invalid);
    let before = f.env.to_snapshot().ledger;
    assert!(MigrationClient::new(&f.env, &guard)
        .try_execute_migration(&invalid)
        .is_err());
    assert_eq!(before, f.env.to_snapshot().ledger);
}

#[test]
fn changed_destination_code_or_strategy_rejects_before_spending() {
    for strategy in [false, true] {
        let (f, destination, guard, draft) = setup(true);
        if strategy {
            let mut first = vault::Client::new(&f.env, &destination)
                .get_assets()
                .get(0)
                .unwrap();
            first.strategies = vec![
                &f.env,
                vault::Strategy {
                    address: Address::generate(&f.env),
                    name: String::from_str(&f.env, "Unsupported strategy"),
                    paused: false,
                },
            ];
            f.env.as_contract(&destination, || {
                f.env
                    .storage()
                    .instance()
                    .set(&VaultFixtureKey::AssetStrategySet(0), &first)
            });
        } else {
            let hash = f.env.deployer().upload_contract_wasm(router::WASM);
            f.env.as_contract(&destination, || {
                f.env.deployer().update_current_contract_wasm(hash)
            });
        }
        auth(&f, &guard, &draft);
        let before = f.env.to_snapshot().ledger;
        assert_eq!(
            MigrationClient::new(&f.env, &guard).try_execute_migration(&draft),
            Err(Ok(soroban_sdk::Error::from_contract_error(1007)))
        );
        assert_eq!(before, f.env.to_snapshot().ledger);
    }
}

#[test]
fn malformed_deposit_vectors_and_wrong_recipient_roll_back() {
    let (f, _, guard, draft) = setup(true);
    for case in 0..3 {
        let mut bad = draft.clone();
        match case {
            0 => bad.min_deposit = vec![&f.env, 1],
            1 => bad.min_deposit = vec![&f.env, 0, 1],
            _ => bad.exit.recipient = Address::generate(&f.env),
        }
        auth(&f, &guard, &bad);
        let before = f.env.to_snapshot().ledger;
        assert!(MigrationClient::new(&f.env, &guard)
            .try_execute_migration(&bad)
            .is_err());
        assert_eq!(before, f.env.to_snapshot().ledger);
    }
}
