use mierkhat_guard::{AssetRoute, CodePin, Error, ExitIntent, GuardClient, Key, RouteProfile};
use soroban_sdk::{
    testutils::{storage::Temporary, Address as _, Ledger, MockAuth, MockAuthInvoke},
    token, vec, Address, BytesN, Env, IntoVal, Map, String, Vec,
};

mod vault {
    soroban_sdk::contractimport!(file = "../artifacts/defindex_vault.wasm");
}
mod factory {
    soroban_sdk::contractimport!(file = "../artifacts/soroswap_factory.wasm");
}
mod router {
    soroban_sdk::contractimport!(file = "../artifacts/soroswap_router.wasm");
}
mod pair {
    soroban_sdk::contractimport!(file = "../artifacts/soroswap_pair.wasm");
}
const GUARD: &[u8] = include_bytes!("../../target/wasm32v1-none/release/mierkhat_guard.wasm");

struct Fixture {
    env: Env,
    owner: Address,
    assets: Vec<Address>,
    settlement: Address,
    vault: Address,
    guard: Address,
    profile: RouteProfile,
    intent: ExitIntent,
}
fn pin(env: &Env, address: &Address, wasm: &[u8]) -> CodePin {
    CodePin {
        address: address.clone(),
        wasm_hash: env
            .crypto()
            .sha256(&soroban_sdk::Bytes::from_slice(env, wasm))
            .into(),
    }
}
fn fixture(settlement_underlying: bool, one_asset: bool) -> Fixture {
    let env = Env::new_with_config(soroban_sdk::testutils::EnvTestConfig {
        capture_snapshot_at_drop: false,
        ..Default::default()
    });
    env.cost_estimate().budget().reset_unlimited();
    env.ledger().with_mut(|l| {
        l.timestamp = 1_000;
        l.sequence_number = 100;
    });
    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    let a = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let b = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let out = if settlement_underlying {
        b.clone()
    } else {
        env.register_stellar_asset_contract_v2(admin.clone())
            .address()
    };
    let factory_address = env.register(factory::WASM, ());
    let f = factory::Client::new(&env, &factory_address);
    let pair_hash = env.deployer().upload_contract_wasm(pair::WASM);
    f.initialize(&admin, &pair_hash);
    let router_address = env.register(router::WASM, ());
    let r = router::Client::new(&env, &router_address);
    r.initialize(&factory_address);
    let assets = if one_asset {
        vec![&env, a.clone()]
    } else {
        vec![&env, a.clone(), b.clone()]
    };
    // Broad auth is ONLY used to seed SAC balances and upstream liquidity/deposit setup.
    env.mock_all_auths();
    for asset in vec![&env, a.clone(), b.clone(), out.clone()].iter() {
        token::StellarAssetClient::new(&env, &asset).mint(&owner, &1_000_000_000_000i128);
    }
    let mut routes = Vec::new(&env);
    for asset in assets.iter() {
        if asset == out {
            routes.push_back(AssetRoute {
                asset,
                pair: vec![&env],
            });
            continue;
        }
        r.add_liquidity(
            &asset,
            &out,
            &100_000_000_000,
            &100_000_000_000,
            &1,
            &1,
            &owner,
            &2_000,
        );
        let address = r.router_pair_for(&asset, &out);
        routes.push_back(AssetRoute {
            asset,
            pair: vec![&env, pin(&env, &address, pair::WASM)],
        });
    }
    let configs = assets
        .iter()
        .map(|a| vault::AssetStrategySet {
            address: a,
            strategies: vec![&env],
        })
        .collect::<std::vec::Vec<_>>();
    let configs = Vec::from_slice(&env, &configs);
    let roles = Map::from_array(
        &env,
        [
            (0u32, admin.clone()),
            (1, admin.clone()),
            (2, admin.clone()),
            (3, admin.clone()),
        ],
    );
    let metadata = Map::from_array(
        &env,
        [
            (
                String::from_str(&env, "name"),
                String::from_str(&env, "Fixture"),
            ),
            (
                String::from_str(&env, "symbol"),
                String::from_str(&env, "MVT"),
            ),
        ],
    );
    let vault_address = env.register(
        vault::WASM,
        (
            configs,
            roles,
            0u32,
            admin,
            0u32,
            router_address.clone(),
            metadata,
            false,
        ),
    );
    let v = vault::Client::new(&env, &vault_address);
    let amounts = if one_asset {
        vec![&env, 10_000_000_000i128]
    } else {
        vec![&env, 10_000_000_000i128, 10_000_000_000i128]
    };
    v.deposit(&amounts, &amounts, &owner, &false);
    let profile = RouteProfile {
        schema: 1,
        network_id: env.ledger().network_id(),
        vault: pin(&env, &vault_address, vault::WASM),
        router: pin(&env, &router_address, router::WASM),
        factory: pin(&env, &factory_address, factory::WASM),
        settlement: out.clone(),
        assets: routes,
        max_seconds: 300,
        max_ledgers: 64,
    };
    let guard = env.register(GUARD, (profile.clone(),));
    let min_withdraw = assets.iter().map(|_| 1i128).collect::<std::vec::Vec<_>>();
    let min_swap = assets
        .iter()
        .map(|a| if a == out { 0i128 } else { 1i128 })
        .collect::<std::vec::Vec<_>>();
    let intent = ExitIntent {
        schema: 1,
        network_id: profile.network_id.clone(),
        guard: guard.clone(),
        profile_hash: GuardClient::new(&env, &guard).profile_hash(),
        intent_id: BytesN::from_array(&env, &[7; 32]),
        owner: owner.clone(),
        recipient: owner.clone(),
        shares: 1_000_000_000,
        min_withdraw: Vec::from_slice(&env, &min_withdraw),
        min_swap: Vec::from_slice(&env, &min_swap),
        min_receive: 1,
        valid_before_time: 1_120,
        valid_before_ledger: 124,
    };
    // Reset to strict auth: the guard and nested invoker auth must work with only this owner tree.
    env.mock_auths(&[]);
    Fixture {
        env,
        owner,
        assets,
        settlement: out,
        vault: vault_address,
        guard,
        profile,
        intent,
    }
}
fn authorize(f: &Fixture, i: &ExitIntent) {
    f.env.mock_auths(&[MockAuth {
        address: &f.owner,
        invoke: &MockAuthInvoke {
            contract: &f.guard,
            fn_name: "execute_exit",
            args: (i.clone(),).into_val(&f.env),
            sub_invokes: &[MockAuthInvoke {
                contract: &f.vault,
                fn_name: "transfer",
                args: (f.owner.clone(), f.guard.clone(), i.shares).into_val(&f.env),
                sub_invokes: &[],
            }],
        },
    }]);
}
#[test]
fn composed_real_wasm_preserves_balances_and_authorization() {
    for (settlement, one) in [(false, false), (true, false), (false, true)] {
        let f = fixture(settlement, one);
        authorize(&f, &f.intent);
        let s0 = token::Client::new(&f.env, &f.vault).balance(&f.owner);
        let out0 = token::Client::new(&f.env, &f.settlement).balance(&f.owner);
        let existing = f
            .assets
            .iter()
            .map(|a| token::Client::new(&f.env, &a).balance(&f.owner))
            .collect::<std::vec::Vec<_>>();
        let receipt = GuardClient::new(&f.env, &f.guard).execute_exit(&f.intent);
        assert_eq!(receipt.shares_spent, f.intent.shares);
        assert!(receipt.received > 0);
        assert_eq!(
            token::Client::new(&f.env, &f.vault).balance(&f.owner),
            s0 - f.intent.shares
        );
        assert_eq!(
            token::Client::new(&f.env, &f.settlement).balance(&f.owner),
            out0 + receipt.received
        );
        for (n, a) in f.assets.iter().enumerate() {
            assert_eq!(token::Client::new(&f.env, &a).balance(&f.guard), 0);
            if a != f.settlement {
                assert_eq!(
                    token::Client::new(&f.env, &a).balance(&f.owner),
                    existing[n]
                );
            }
        }
        f.env.as_contract(&f.guard, || {
            let key = Key::Used(f.owner.clone(), f.intent.intent_id.clone());
            assert!(f.env.storage().temporary().has(&key));
            assert!(f.env.storage().temporary().get_ttl(&key) >= 25);
        });
    }
}
#[test]
fn final_minimum_rolls_back_every_ledger_entry_after_both_swaps() {
    let f = fixture(false, false);
    let mut i = f.intent.clone();
    i.min_receive = 100_000_000_000;
    authorize(&f, &i);
    let before = f.env.to_snapshot().ledger;
    assert_eq!(
        GuardClient::new(&f.env, &f.guard).try_execute_exit(&i),
        Err(Ok(soroban_sdk::Error::from_contract_error(
            Error::OutcomeBelowMinimum as u32
        )))
    );
    assert_eq!(before, f.env.to_snapshot().ledger);
    // Marker also rolled back: same intent succeeds with reviewed attainable bounds.
    authorize(&f, &f.intent);
    GuardClient::new(&f.env, &f.guard).execute_exit(&f.intent);
}
#[test]
fn missing_root_authority_fails_without_ledger_changes() {
    let f = fixture(false, false);
    let before = f.env.to_snapshot().ledger;
    assert!(GuardClient::new(&f.env, &f.guard)
        .try_execute_exit(&f.intent)
        .is_err());
    assert_eq!(before, f.env.to_snapshot().ledger);
}
#[test]
fn replay_deadlines_vectors_and_donations() {
    let f = fixture(false, false);
    // Setup donations explicitly under owner transfer auth; never used during guard execution.
    for a in f
        .assets
        .iter()
        .chain(core::iter::once(f.settlement.clone()))
        .chain(core::iter::once(f.vault.clone()))
    {
        token::Client::new(&f.env, &a)
            .mock_auths(&[MockAuth {
                address: &f.owner,
                invoke: &MockAuthInvoke {
                    contract: &a,
                    fn_name: "transfer",
                    args: (f.owner.clone(), f.guard.clone(), 12345i128).into_val(&f.env),
                    sub_invokes: &[],
                },
            }])
            .transfer(&f.owner, &f.guard, &12345);
    }
    authorize(&f, &f.intent);
    GuardClient::new(&f.env, &f.guard).execute_exit(&f.intent);
    for a in f
        .assets
        .iter()
        .chain(core::iter::once(f.settlement.clone()))
        .chain(core::iter::once(f.vault.clone()))
    {
        assert_eq!(token::Client::new(&f.env, &a).balance(&f.guard), 12345);
    }
    assert_eq!(
        GuardClient::new(&f.env, &f.guard).try_execute_exit(&f.intent),
        Err(Ok(soroban_sdk::Error::from_contract_error(
            Error::AlreadyExecuted as u32
        )))
    );
    let mut i = f.intent.clone();
    i.intent_id = BytesN::from_array(&f.env, &[9; 32]);
    i.valid_before_time = 1_000;
    assert_eq!(
        GuardClient::new(&f.env, &f.guard).try_execute_exit(&i),
        Err(Ok(soroban_sdk::Error::from_contract_error(
            Error::Expired as u32
        )))
    );
    i = f.intent.clone();
    i.min_withdraw = vec![&f.env];
    assert_eq!(
        GuardClient::new(&f.env, &f.guard).try_execute_exit(&i),
        Err(Ok(soroban_sdk::Error::from_contract_error(
            Error::BadSchema as u32
        )))
    );
}

#[test]
fn signed_field_mutations_reject_and_do_not_write() {
    let f = fixture(false, false);
    let mut cases = std::vec::Vec::new();
    let mut i = f.intent.clone();
    i.network_id = BytesN::from_array(&f.env, &[99; 32]);
    cases.push(i);
    let mut i = f.intent.clone();
    i.guard = f.vault.clone();
    cases.push(i);
    let mut i = f.intent.clone();
    i.profile_hash = BytesN::from_array(&f.env, &[0; 32]);
    cases.push(i);
    let mut i = f.intent.clone();
    i.recipient = Address::generate(&f.env);
    cases.push(i);
    let mut i = f.intent.clone();
    i.shares = 0;
    cases.push(i);
    let mut i = f.intent.clone();
    i.min_receive = -1;
    cases.push(i);
    let mut i = f.intent.clone();
    i.min_swap = vec![&f.env, 0i128, 1i128];
    cases.push(i);
    let mut i = f.intent.clone();
    i.valid_before_ledger = 100;
    cases.push(i);
    let mut i = f.intent.clone();
    i.valid_before_ledger = 165;
    cases.push(i);
    let mut i = f.intent.clone();
    i.valid_before_time = 1301;
    cases.push(i);
    for i in cases {
        authorize(&f, &i);
        let before = f.env.to_snapshot().ledger;
        assert!(GuardClient::new(&f.env, &f.guard)
            .try_execute_exit(&i)
            .is_err());
        assert_eq!(before, f.env.to_snapshot().ledger);
    }
    // R03: owner authorization over only the share transfer cannot authorize root execution.
    token::Client::new(&f.env, &f.vault).mock_auths(&[MockAuth {
        address: &f.owner,
        invoke: &MockAuthInvoke {
            contract: &f.vault,
            fn_name: "transfer",
            args: (f.owner.clone(), f.guard.clone(), f.intent.shares).into_val(&f.env),
            sub_invokes: &[],
        },
    }]);
    assert!(GuardClient::new(&f.env, &f.guard)
        .try_execute_exit(&f.intent)
        .is_err());
}
#[test]
fn second_swap_failure_restores_full_state_and_donations_cannot_fund_minimum() {
    let f = fixture(false, false);
    let mut i = f.intent.clone();
    i.min_swap = vec![&f.env, 1i128, i128::MAX];
    authorize(&f, &i);
    let before = f.env.to_snapshot().ledger;
    assert!(GuardClient::new(&f.env, &f.guard)
        .try_execute_exit(&i)
        .is_err());
    assert_eq!(before, f.env.to_snapshot().ledger);
    let a = &f.settlement;
    token::Client::new(&f.env, a)
        .mock_auths(&[MockAuth {
            address: &f.owner,
            invoke: &MockAuthInvoke {
                contract: a,
                fn_name: "transfer",
                args: (f.owner.clone(), f.guard.clone(), 10_000_000_000i128).into_val(&f.env),
                sub_invokes: &[],
            },
        }])
        .transfer(&f.owner, &f.guard, &10_000_000_000);
    let mut i = f.intent.clone();
    i.min_receive = 5_000_000_000;
    authorize(&f, &i);
    let before = f.env.to_snapshot().ledger;
    assert_eq!(
        GuardClient::new(&f.env, &f.guard).try_execute_exit(&i),
        Err(Ok(soroban_sdk::Error::from_contract_error(
            Error::OutcomeBelowMinimum as u32
        )))
    );
    assert_eq!(before, f.env.to_snapshot().ledger);
}
#[test]
fn runtime_code_pin_change_rejects_before_spending() {
    let f = fixture(false, false);
    let hash = f.env.deployer().upload_contract_wasm(router::WASM);
    f.env.as_contract(&f.vault, || {
        f.env.deployer().update_current_contract_wasm(hash)
    });
    authorize(&f, &f.intent);
    let before = f.env.to_snapshot().ledger;
    assert_eq!(
        GuardClient::new(&f.env, &f.guard).try_execute_exit(&f.intent),
        Err(Ok(soroban_sdk::Error::from_contract_error(
            Error::DependencyChanged as u32
        )))
    );
    assert_eq!(before, f.env.to_snapshot().ledger);
}
#[test]
fn cross_language_vectors_and_host_resource_observation() {
    use soroban_sdk::xdr::ToXdr;
    let f = fixture(false, false);
    authorize(&f, &f.intent);
    let receipt = GuardClient::new(&f.env, &f.guard).execute_exit(&f.intent);
    let resources = f.env.cost_estimate().resources();
    let hex = |bytes: soroban_sdk::Bytes| -> std::string::String {
        bytes.iter().map(|b| std::format!("{b:02x}")).collect()
    };
    let output=std::format!("{{\n  \"kind\": \"strict owner mock host fixture, not a signed ledger transaction\",\n  \"profile_xdr_hex\": \"{}\",\n  \"intent_xdr_hex\": \"{}\",\n  \"receipt_xdr_hex\": \"{}\",\n  \"resources_debug\": \"{:?}\"\n}}\n",hex(f.profile.to_xdr(&f.env)),hex(f.intent.to_xdr(&f.env)),hex(receipt.to_xdr(&f.env)),resources);
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../evidence/host-fixture.json");
    std::fs::write(path, output).unwrap();
}

#[test]
fn exclusive_deadlines_and_expired_marker_cannot_revive_an_intent() {
    let f = fixture(false, false);
    let mut i = f.intent.clone();
    i.valid_before_ledger = 101;
    i.valid_before_time = 1001;
    authorize(&f, &i);
    GuardClient::new(&f.env, &f.guard).execute_exit(&i);
    f.env.as_contract(&f.guard, || {
        f.env
            .storage()
            .temporary()
            .remove(&Key::Used(f.owner.clone(), i.intent_id.clone()))
    });
    f.env.ledger().with_mut(|l| l.sequence_number = 101);
    assert_eq!(
        GuardClient::new(&f.env, &f.guard).try_execute_exit(&i),
        Err(Ok(soroban_sdk::Error::from_contract_error(
            Error::Expired as u32
        )))
    );
    let f = fixture(false, false);
    let mut i = f.intent.clone();
    i.valid_before_time = 1000;
    assert_eq!(
        GuardClient::new(&f.env, &f.guard).try_execute_exit(&i),
        Err(Ok(soroban_sdk::Error::from_contract_error(
            Error::Expired as u32
        )))
    );
}

#[soroban_sdk::contract]
struct AuthorizationProbe;
#[soroban_sdk::contractimpl]
impl AuthorizationProbe {
    pub fn run(
        env: Env,
        router: Address,
        input: Address,
        output: Address,
        pair: Address,
        amount: i128,
        authorized_amount: i128,
        authorized_token: Address,
    ) {
        use soroban_sdk::auth::{ContractContext, InvokerContractAuthEntry, SubContractInvocation};
        let me = env.current_contract_address();
        env.authorize_as_current_contract(vec![
            &env,
            InvokerContractAuthEntry::Contract(SubContractInvocation {
                context: ContractContext {
                    contract: authorized_token,
                    fn_name: soroban_sdk::symbol_short!("transfer"),
                    args: (me.clone(), pair, authorized_amount).into_val(&env),
                },
                sub_invocations: vec![&env],
            }),
        ]);
        mierkhat_guard::RouterClient::new(&env, &router).swap_exact_tokens_for_tokens(
            &amount,
            &1,
            &vec![&env, input, output],
            &me,
            &1120,
        );
    }
}
#[test]
fn nested_transfer_authorization_rejects_wrong_amount_and_token() {
    let f = fixture(false, false);
    let probe = f.env.register(AuthorizationProbe, ());
    let input = f.assets.get(0).unwrap();
    let pair = f
        .profile
        .assets
        .get(0)
        .unwrap()
        .pair
        .first()
        .unwrap()
        .address;
    token::Client::new(&f.env, &input)
        .mock_auths(&[MockAuth {
            address: &f.owner,
            invoke: &MockAuthInvoke {
                contract: &input,
                fn_name: "transfer",
                args: (f.owner.clone(), probe.clone(), 1_000_000i128).into_val(&f.env),
                sub_invokes: &[],
            },
        }])
        .transfer(&f.owner, &probe, &1_000_000);
    f.env.mock_auths(&[]);
    let client = AuthorizationProbeClient::new(&f.env, &probe);
    for (amount, token) in [
        (999_999i128, input.clone()),
        (1_000_000, f.settlement.clone()),
    ] {
        let before = f.env.to_snapshot().ledger;
        assert!(client
            .try_run(
                &f.profile.router.address,
                &input,
                &f.settlement,
                &pair,
                &1_000_000,
                &amount,
                &token
            )
            .is_err());
        assert_eq!(before, f.env.to_snapshot().ledger);
    }
    client.run(
        &f.profile.router.address,
        &input,
        &f.settlement,
        &pair,
        &1_000_000,
        &1_000_000,
        &input,
    );
    assert_eq!(token::Client::new(&f.env, &input).balance(&probe), 0);
}

// Exact storage-key encoding from the pinned upstream ABI. Only fixture state
// is changed; get_assets, swaps and guard execution still use the real WASM.
#[soroban_sdk::contracttype]
#[derive(Clone)]
enum VaultFixtureKey {
    TotalAssets,
    AssetStrategySet(u32),
}
#[soroban_sdk::contracttype]
#[derive(Clone)]
enum RouterFixtureKey {
    Factory,
}

#[test]
fn vault_configuration_mutations_reject_before_spending_and_preserve_all_state() {
    for case in 0..6 {
        let f = fixture(false, false);
        let mut assets = vault::Client::new(&f.env, &f.vault).get_assets();
        let mut first = assets.get(0).unwrap();
        f.env.as_contract(&f.vault, || match case {
            0 => f
                .env
                .storage()
                .instance()
                .set(&VaultFixtureKey::TotalAssets, &1u32),
            1 => {
                let second = assets.get(1).unwrap();
                f.env
                    .storage()
                    .instance()
                    .set(&VaultFixtureKey::AssetStrategySet(0), &second);
                f.env
                    .storage()
                    .instance()
                    .set(&VaultFixtureKey::AssetStrategySet(1), &first);
            }
            2 => {
                first.address = f.settlement.clone();
                f.env
                    .storage()
                    .instance()
                    .set(&VaultFixtureKey::AssetStrategySet(0), &first);
            }
            3 | 4 => {
                first.strategies = vec![
                    &f.env,
                    vault::Strategy {
                        address: Address::generate(&f.env),
                        name: String::from_str(&f.env, "Rejected fixture strategy"),
                        paused: case == 4,
                    },
                ];
                f.env
                    .storage()
                    .instance()
                    .set(&VaultFixtureKey::AssetStrategySet(0), &first);
            }
            _ => {
                assets.set(1, first.clone());
                f.env
                    .storage()
                    .instance()
                    .set(&VaultFixtureKey::AssetStrategySet(1), &first);
            }
        });
        authorize(&f, &f.intent);
        let before = f.env.to_snapshot().ledger;
        assert_eq!(
            GuardClient::new(&f.env, &f.guard).try_execute_exit(&f.intent),
            Err(Ok(soroban_sdk::Error::from_contract_error(
                Error::DependencyChanged as u32
            ))),
            "configuration case {case}",
        );
        assert_eq!(
            before,
            f.env.to_snapshot().ledger,
            "configuration case {case}"
        );
    }
}

#[test]
fn router_factory_and_each_executable_pin_change_rejects_before_spending() {
    for case in 0..5 {
        let f = fixture(false, false);
        if case == 0 {
            f.env.as_contract(&f.profile.router.address, || {
                f.env
                    .storage()
                    .instance()
                    .set(&RouterFixtureKey::Factory, &Address::generate(&f.env));
            });
        } else {
            let address = match case {
                1 => f.profile.factory.address.clone(),
                2 => f.profile.router.address.clone(),
                _ => {
                    f.profile
                        .assets
                        .get(case - 3)
                        .unwrap()
                        .pair
                        .get(0)
                        .unwrap()
                        .address
                }
            };
            let replacement = f.env.deployer().upload_contract_wasm(vault::WASM);
            f.env.as_contract(&address, || {
                f.env.deployer().update_current_contract_wasm(replacement)
            });
        }
        authorize(&f, &f.intent);
        let before = f.env.to_snapshot().ledger;
        assert_eq!(
            GuardClient::new(&f.env, &f.guard).try_execute_exit(&f.intent),
            Err(Ok(soroban_sdk::Error::from_contract_error(
                Error::DependencyChanged as u32
            ))),
            "dependency case {case}",
        );
        assert_eq!(before, f.env.to_snapshot().ledger, "dependency case {case}");
    }
}

#[test]
fn corrupted_pair_asset_state_rolls_back_all_touched_state() {
    for token_key in [0u32, 1u32] {
        let f = fixture(false, false);
        let address = f
            .profile
            .assets
            .get(0)
            .unwrap()
            .pair
            .get(0)
            .unwrap()
            .address;
        let unrelated = f
            .env
            .register_stellar_asset_contract_v2(Address::generate(&f.env))
            .address();
        // Pinned Soroswap pair storage uses integer token keys 0 and 1.
        f.env.as_contract(&address, || {
            f.env.storage().instance().set(&token_key, &unrelated)
        });
        authorize(&f, &f.intent);
        let before = f.env.to_snapshot().ledger;
        assert!(GuardClient::new(&f.env, &f.guard)
            .try_execute_exit(&f.intent)
            .is_err());
        assert_eq!(
            before,
            f.env.to_snapshot().ledger,
            "pair token key {token_key}"
        );
    }
}
