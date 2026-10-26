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
    let env = Env::default();
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
