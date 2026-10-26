#![no_std]

mod types;
use soroban_sdk::{
    auth::{ContractContext, InvokerContractAuthEntry, SubContractInvocation},
    contract, contractclient, contractimpl, panic_with_error, symbol_short, token, vec,
    xdr::ToXdr,
    Address, BytesN, Env, Executable, IntoVal, Symbol, Vec,
};
pub use types::*;

#[contractclient(name = "VaultClient")]
pub trait Vault {
    fn get_assets(env: Env) -> Vec<AssetStrategySet>;
    fn withdraw(
        env: Env,
        withdraw_shares: i128,
        min_amounts_out: Vec<i128>,
        from: Address,
    ) -> Vec<i128>;
}
#[contractclient(name = "RouterClient")]
pub trait Router {
    fn get_factory(env: Env) -> Address;
    fn router_pair_for(env: Env, token_a: Address, token_b: Address) -> Address;
    fn swap_exact_tokens_for_tokens(
        env: Env,
        amount_in: i128,
        amount_out_min: i128,
        path: Vec<Address>,
        to: Address,
        deadline: u64,
    ) -> Vec<i128>;
}

fn ensure(env: &Env, ok: bool, error: Error) {
    if !ok {
        panic_with_error!(env, error);
    }
}
fn add(env: &Env, a: i128, b: i128) -> i128 {
    a.checked_add(b)
        .unwrap_or_else(|| panic_with_error!(env, Error::Arithmetic))
}
fn sub(env: &Env, a: i128, b: i128) -> i128 {
    a.checked_sub(b)
        .unwrap_or_else(|| panic_with_error!(env, Error::Arithmetic))
}
fn balance(env: &Env, asset: &Address, owner: &Address) -> i128 {
    let value = token::Client::new(env, asset).balance(owner);
    ensure(env, value >= 0, Error::BadAmount);
    value
}
fn code(env: &Env, pin: &CodePin) {
    ensure(
        env,
        pin.address.executable() == Some(Executable::Wasm(pin.wasm_hash.clone())),
        Error::DependencyChanged,
    );
}
fn sac(env: &Env, asset: &Address) {
    ensure(
        env,
        asset.executable() == Some(Executable::StellarAsset),
        Error::UnsupportedIdentity,
    );
}
fn dependencies(env: &Env, p: &RouteProfile) {
    let guard = env.current_contract_address();
    ensure(
        env,
        p.schema == 1 && !p.assets.is_empty() && p.assets.len() <= 2,
        Error::BadSchema,
    );
    ensure(
        env,
        p.network_id == env.ledger().network_id(),
        Error::WrongDomain,
    );
    ensure(
        env,
        p.max_seconds == 300 && p.max_ledgers == 64,
        Error::DeadlineTooFar,
    );
    let ids = vec![
        env,
        guard,
        p.vault.address.clone(),
        p.router.address.clone(),
        p.factory.address.clone(),
    ];
    for i in 0..ids.len() {
        for j in 0..i {
            ensure(env, ids.get(i) != ids.get(j), Error::UnsupportedIdentity);
        }
    }
    sac(env, &p.settlement);
    ensure(
        env,
        !ids.contains(&p.settlement),
        Error::UnsupportedIdentity,
    );
    code(env, &p.vault);
    code(env, &p.router);
    code(env, &p.factory);
    let assets = VaultClient::new(env, &p.vault.address).get_assets();
    ensure(
        env,
        assets.len() == p.assets.len(),
        Error::DependencyChanged,
    );
    let router = RouterClient::new(env, &p.router.address);
    ensure(
        env,
        router.get_factory() == p.factory.address,
        Error::DependencyChanged,
    );
    let mut pair_ids = Vec::new(env);
    let mut asset_ids = Vec::new(env);
    for (i, r) in p.assets.iter().enumerate() {
        sac(env, &r.asset);
        ensure(
            env,
            !ids.contains(&r.asset) && !asset_ids.contains(&r.asset),
            Error::UnsupportedIdentity,
        );
        asset_ids.push_back(r.asset.clone());
        let actual = assets.get(i as u32).unwrap();
        ensure(
            env,
            actual.address == r.asset && actual.strategies.is_empty(),
            Error::DependencyChanged,
        );
        ensure(env, r.pair.len() <= 1, Error::BadSchema);
        match r.pair.first() {
            None => ensure(env, r.asset == p.settlement, Error::UnsupportedIdentity),
            Some(ref pin) => {
                ensure(
                    env,
                    r.asset != p.settlement
                        && !ids.contains(&pin.address)
                        && !pair_ids.contains(&pin.address)
                        && pin.address != p.settlement,
                    Error::UnsupportedIdentity,
                );
                for a in p.assets.iter() {
                    ensure(env, a.asset != pin.address, Error::UnsupportedIdentity);
                }
                pair_ids.push_back(pin.address.clone());
                code(env, pin);
                ensure(
                    env,
                    router.router_pair_for(&r.asset, &p.settlement) == pin.address,
                    Error::DependencyChanged,
                );
            }
        }
    }
}
fn maintain(env: &Env) {
    // Five second cadence is a TTL target, never a deadline oracle.
    let max = env.storage().max_ttl();
    env.storage()
        .instance()
        .extend_ttl(core::cmp::min(120_960, max), core::cmp::min(518_400, max));
}

#[contract]
pub struct Guard;
#[contractimpl]
impl Guard {
    pub fn __constructor(env: Env, profile: RouteProfile) {
        dependencies(&env, &profile);
        let digest: BytesN<32> = env.crypto().sha256(&profile.clone().to_xdr(&env)).into();
        env.storage().instance().set(&Key::Profile, &profile);
        env.storage().instance().set(&Key::Digest, &digest);
        maintain(&env);
    }
    pub fn profile(env: Env) -> RouteProfile {
        env.storage().instance().get(&Key::Profile).unwrap()
    }
    pub fn profile_hash(env: Env) -> BytesN<32> {
        env.storage().instance().get(&Key::Digest).unwrap()
    }
    pub fn extend_life(env: Env) {
        maintain(&env);
    }

    pub fn execute_exit(env: Env, intent: ExitIntent) -> ExitReceipt {
        let p = Self::profile(env.clone());
        let digest = Self::profile_hash(env.clone());
        let me = env.current_contract_address();
        ensure(
            &env,
            intent.schema == 1
                && intent.min_withdraw.len() == p.assets.len()
                && intent.min_swap.len() == p.assets.len(),
            Error::BadSchema,
        );
        ensure(
            &env,
            intent.network_id == env.ledger().network_id()
                && intent.network_id == p.network_id
                && intent.guard == me
                && intent.profile_hash == digest,
            Error::WrongDomain,
        );
        ensure(
            &env,
            intent.owner == intent.recipient,
            Error::UnsupportedRecipient,
        );
        ensure(
            &env,
            intent.owner != me
                && intent.owner != p.vault.address
                && intent.owner != p.router.address
                && intent.owner != p.factory.address,
            Error::UnsupportedIdentity,
        );
        let now = env.ledger().timestamp();
        let ledger = env.ledger().sequence();
        ensure(
            &env,
            now < intent.valid_before_time && ledger < intent.valid_before_ledger,
            Error::Expired,
        );
        ensure(
            &env,
            intent.valid_before_time
                <= now
                    .checked_add(p.max_seconds)
                    .unwrap_or_else(|| panic_with_error!(&env, Error::Arithmetic))
                && intent.valid_before_ledger
                    <= ledger
                        .checked_add(p.max_ledgers)
                        .unwrap_or_else(|| panic_with_error!(&env, Error::Arithmetic)),
            Error::DeadlineTooFar,
        );
        ensure(
            &env,
            intent.shares > 0 && intent.min_receive > 0,
            Error::BadAmount,
        );
        for (i, r) in p.assets.iter().enumerate() {
            ensure(
                &env,
                intent.min_withdraw.get(i as u32).unwrap() > 0,
                Error::BadAmount,
            );
            let minimum = intent.min_swap.get(i as u32).unwrap();
            ensure(
                &env,
                if r.asset == p.settlement {
                    minimum == 0
                } else {
                    minimum > 0
                },
                Error::BadAmount,
            );
        }
        let key = Key::Used(intent.owner.clone(), intent.intent_id.clone());
        ensure(
            &env,
            !env.storage().temporary().has(&key),
            Error::AlreadyExecuted,
        );
        intent.owner.require_auth();
        dependencies(&env, &p);
        let s0 = balance(&env, &p.vault.address, &intent.owner);
        let g0 = balance(&env, &p.vault.address, &me);
        ensure(&env, s0 >= intent.shares, Error::InsufficientShares);
        let mut tokens = Vec::new(&env);
        for r in p.assets.iter() {
            tokens.push_back(r.asset);
        }
        if !tokens.contains(&p.settlement) {
            tokens.push_back(p.settlement.clone());
        }
        let mut gb = Vec::new(&env);
        let mut ob = Vec::new(&env);
        for a in tokens.iter() {
            gb.push_back(balance(&env, &a, &me));
            ob.push_back(balance(&env, &a, &intent.owner));
        }
        let out0 = balance(&env, &p.settlement, &me);
        let owner_out0 = balance(&env, &p.settlement, &intent.owner);
        // R04/R11: reserve before external calls, retained through exclusive validity plus margin.
        env.storage().temporary().set(&key, &true);
        let ttl = intent
            .valid_before_ledger
            .checked_sub(ledger)
            .and_then(|n| n.checked_add(1))
            .unwrap_or_else(|| panic_with_error!(&env, Error::Arithmetic));
        ensure(&env, ttl <= env.storage().max_ttl(), Error::DeadlineTooFar);
        env.storage().temporary().extend_ttl(&key, ttl, ttl);
        let shares = token::Client::new(&env, &p.vault.address);
        shares.transfer(&intent.owner, &me, &intent.shares);
        ensure(
            &env,
            shares.balance(&intent.owner) == sub(&env, s0, intent.shares)
                && shares.balance(&me) == add(&env, g0, intent.shares),
            Error::UnexpectedBalance,
        );
        let withdrawn = VaultClient::new(&env, &p.vault.address).withdraw(
            &intent.shares,
            &intent.min_withdraw,
            &me,
        );
        ensure(
            &env,
            withdrawn.len() == p.assets.len(),
            Error::WithdrawalMismatch,
        );
        ensure(&env, shares.balance(&me) == g0, Error::UnexpectedBalance);
        // Measure all withdrawals before performing either swap.
        for (i, r) in p.assets.iter().enumerate() {
            let amount = sub(
                &env,
                balance(&env, &r.asset, &me),
                gb.get(i as u32).unwrap(),
            );
            ensure(
                &env,
                amount > 0
                    && amount == withdrawn.get(i as u32).unwrap()
                    && amount >= intent.min_withdraw.get(i as u32).unwrap(),
                Error::WithdrawalMismatch,
            );
        }
        let mut swapped = Vec::new(&env);
        for (i, r) in p.assets.iter().enumerate() {
            if r.asset == p.settlement {
                swapped.push_back(0);
                continue;
            }
            let amount = withdrawn.get(i as u32).unwrap();
            let pair = r.pair.first().unwrap();
            let before = balance(&env, &p.settlement, &me);
            // R10: only this exact nested token transfer is delegated to the router.
            env.authorize_as_current_contract(vec![
                &env,
                InvokerContractAuthEntry::Contract(SubContractInvocation {
                    context: ContractContext {
                        contract: r.asset.clone(),
                        fn_name: symbol_short!("transfer"),
                        args: (me.clone(), pair.address, amount).into_val(&env),
                    },
                    sub_invocations: vec![&env],
                }),
            ]);
            let minimum = intent.min_swap.get(i as u32).unwrap();
            let result = RouterClient::new(&env, &p.router.address).swap_exact_tokens_for_tokens(
                &amount,
                &minimum,
                &vec![&env, r.asset.clone(), p.settlement.clone()],
                &me,
                &intent.valid_before_time,
            );
            let got = sub(&env, balance(&env, &p.settlement, &me), before);
            ensure(
                &env,
                result.len() == 2
                    && result.get(0) == Some(amount)
                    && result.get(1) == Some(got)
                    && got > 0
                    && got >= minimum
                    && balance(&env, &r.asset, &me) == gb.get(i as u32).unwrap(),
                Error::SwapMismatch,
            );
            swapped.push_back(got);
        }
        let received = sub(&env, balance(&env, &p.settlement, &me), out0);
        ensure(
            &env,
            received >= intent.min_receive,
            Error::OutcomeBelowMinimum,
        );
        token::Client::new(&env, &p.settlement).transfer(&me, &intent.owner, &received);
        ensure(
            &env,
            balance(&env, &p.settlement, &intent.owner) == add(&env, owner_out0, received),
            Error::UnexpectedBalance,
        );
        ensure(
            &env,
            shares.balance(&intent.owner) == sub(&env, s0, intent.shares)
                && shares.balance(&me) == g0,
            Error::UnexpectedBalance,
        );
        for (i, a) in tokens.iter().enumerate() {
            ensure(
                &env,
                balance(&env, &a, &me) == gb.get(i as u32).unwrap(),
                Error::UnexpectedBalance,
            );
            if a != p.settlement {
                ensure(
                    &env,
                    balance(&env, &a, &intent.owner) == ob.get(i as u32).unwrap(),
                    Error::UnexpectedBalance,
                );
            }
        }
        dependencies(&env, &p);
        maintain(&env);
        let receipt = ExitReceipt {
            intent_id: intent.intent_id.clone(),
            intent_hash: env.crypto().sha256(&intent.clone().to_xdr(&env)).into(),
            profile_hash: digest,
            owner: intent.owner,
            shares_spent: intent.shares,
            received,
            withdrawn,
            swapped_out: swapped,
            ledger,
            timestamp: now,
        };
        env.events()
            .publish((Symbol::new(&env, "exit_v1"),), &receipt);
        receipt
    }
}
