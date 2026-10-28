#![no_std]

mod types;
use soroban_sdk::{
    auth::{ContractContext, InvokerContractAuthEntry, SubContractInvocation},
    contract, contractclient, contractimpl, panic_with_error, symbol_short, token, vec,
    xdr::ToXdr,
    Address, BytesN, Env, Executable, IntoVal, Map, Symbol, TryFromVal, Val, Vec,
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

fn migration_dependencies(env: &Env, m: &MigrationProfile) {
    dependencies(env, &m.source);
    let p = &m.source;
    let dest = &m.destination.address;
    let me = env.current_contract_address();
    ensure(
        env,
        *dest != p.vault.address
            && *dest != me
            && *dest != p.router.address
            && *dest != p.factory.address
            && *dest != p.settlement
            && !m.destination_assets.is_empty()
            && m.destination_assets.len() <= 2,
        Error::UnsupportedIdentity,
    );
    code(env, &m.destination);
    let actual = VaultClient::new(env, dest).get_assets();
    ensure(
        env,
        actual.len() == m.destination_assets.len(),
        Error::DependencyChanged,
    );
    let router = RouterClient::new(env, &p.router.address);
    let mut seen = Vec::new(env);
    for (i, a) in m.destination_assets.iter().enumerate() {
        sac(env, &a.asset);
        ensure(
            env,
            !seen.contains(&a.asset)
                && a.asset != *dest
                && a.asset != me
                && a.asset != p.vault.address
                && a.asset != p.router.address
                && a.asset != p.factory.address,
            Error::UnsupportedIdentity,
        );
        seen.push_back(a.asset.clone());
        let configured = actual.get(i as u32).unwrap();
        ensure(
            env,
            configured.address == a.asset && configured.strategies.is_empty(),
            Error::DependencyChanged,
        );
        ensure(env, a.pair.len() <= 1, Error::BadSchema);
        match a.pair.first() {
            None => ensure(env, a.asset == p.settlement, Error::UnsupportedIdentity),
            Some(pair) => {
                ensure(
                    env,
                    a.asset != p.settlement
                        && pair.address != *dest
                        && pair.address != me
                        && pair.address != p.vault.address
                        && pair.address != p.router.address
                        && pair.address != p.factory.address
                        && pair.address != p.settlement,
                    Error::UnsupportedIdentity,
                );
                for asset in p.assets.iter().chain(m.destination_assets.iter()) {
                    ensure(
                        env,
                        asset.asset != pair.address && asset.asset != *dest,
                        Error::UnsupportedIdentity,
                    );
                }
                code(env, &pair);
                ensure(
                    env,
                    router.router_pair_for(&p.settlement, &a.asset) == pair.address,
                    Error::DependencyChanged,
                );
            }
        }
    }
    for a in p.assets.iter() {
        ensure(
            env,
            a.asset != *dest && !a.pair.iter().any(|pin| pin.address == *dest),
            Error::UnsupportedIdentity,
        );
    }
}
fn authorize_transfer(env: &Env, asset: &Address, to: &Address, amount: i128) {
    env.authorize_as_current_contract(vec![
        env,
        InvokerContractAuthEntry::Contract(SubContractInvocation {
            context: ContractContext {
                contract: asset.clone(),
                fn_name: symbol_short!("transfer"),
                args: (env.current_contract_address(), to.clone(), amount).into_val(env),
            },
            sub_invocations: vec![env],
        }),
    ]);
}
// Exact arithmetic of the pinned strategy-free vault's proportional deposit.
// Read immediately before deposit; no authorization is granted for a larger transfer.
fn deposit_amounts(
    env: &Env,
    destination: &Address,
    desired: &Vec<i128>,
    minimums: &Vec<i128>,
) -> Vec<i128> {
    let supply: i128 =
        env.invoke_contract(destination, &Symbol::new(env, "total_supply"), vec![env]);
    if desired.len() == 1 || supply == 0 {
        return desired.clone();
    }
    let managed: Vec<Map<Symbol, Val>> = env.invoke_contract(
        destination,
        &Symbol::new(env, "fetch_total_managed_funds"),
        vec![env],
    );
    ensure(
        env,
        managed.len() == desired.len(),
        Error::DependencyChanged,
    );
    let mut reserves = Vec::new(env);
    for row in managed.iter() {
        let amount =
            i128::try_from_val(env, &row.get(Symbol::new(env, "total_amount")).unwrap()).unwrap();
        ensure(env, amount >= 0, Error::BadAmount);
        reserves.push_back(amount);
    }
    for i in 0..desired.len() {
        let reserve = reserves.get(i).unwrap();
        if reserve == 0 {
            continue;
        }
        let target = desired.get(i).unwrap();
        let mut candidate = Vec::new(env);
        let mut valid = target > 0;
        for j in 0..desired.len() {
            let value = if i == j {
                target
            } else {
                let product = reserves
                    .get(j)
                    .unwrap()
                    .checked_mul(target)
                    .unwrap_or_else(|| panic_with_error!(env, Error::Arithmetic));
                let floor = product.checked_div(reserve).unwrap();
                add(env, floor, if product % reserve == 0 { 0 } else { 1 })
            };
            valid = valid && value >= minimums.get(j).unwrap() && value <= desired.get(j).unwrap();
            candidate.push_back(value);
        }
        if valid {
            return candidate;
        }
    }
    panic_with_error!(env, Error::DepositMismatch);
}
fn deposit_and_refund(
    env: &Env,
    m: &MigrationProfile,
    draft: &MigrationIntent,
    tokens: &Vec<Address>,
    gb: &Vec<i128>,
    ob: &Vec<i128>,
    dg0: i128,
    do0: i128,
    received: i128,
) -> (i128, Vec<i128>, Vec<i128>, Vec<i128>) {
    let p = &m.source;
    let me = env.current_contract_address();
    let mut spent = 0;
    for (i, asset) in m.destination_assets.iter().enumerate() {
        let amount = draft.convert_inputs.get(i as u32).unwrap();
        ensure(
            env,
            draft.min_deposit.get(i as u32).unwrap() > 0 && amount >= 0,
            Error::BadAmount,
        );
        if asset.asset == p.settlement {
            ensure(env, amount == 0, Error::BadAmount);
        } else {
            ensure(env, amount > 0, Error::BadAmount);
            spent = add(env, spent, amount);
        }
    }
    ensure(env, spent <= received, Error::BadAmount);
    let mut converted = Vec::new(env);
    for (i, asset) in m.destination_assets.iter().enumerate() {
        let amount = draft.convert_inputs.get(i as u32).unwrap();
        if asset.asset == p.settlement {
            converted.push_back(0);
            continue;
        }
        let before = balance(env, &asset.asset, &me);
        authorize_transfer(
            env,
            &p.settlement,
            &asset.pair.first().unwrap().address,
            amount,
        );
        let result = RouterClient::new(env, &p.router.address).swap_exact_tokens_for_tokens(
            &amount,
            &draft.min_deposit.get(i as u32).unwrap(),
            &vec![env, p.settlement.clone(), asset.asset.clone()],
            &me,
            &draft.exit.valid_before_time,
        );
        let got = sub(env, balance(env, &asset.asset, &me), before);
        ensure(
            env,
            result.len() == 2
                && result.get(0) == Some(amount)
                && result.get(1) == Some(got)
                && got >= draft.min_deposit.get(i as u32).unwrap(),
            Error::SwapMismatch,
        );
        converted.push_back(got);
    }
    let mut desired = Vec::new(env);
    for (i, asset) in m.destination_assets.iter().enumerate() {
        let index = tokens.first_index_of(&asset.asset).unwrap();
        let available = sub(env, balance(env, &asset.asset, &me), gb.get(index).unwrap());
        ensure(
            env,
            available >= draft.min_deposit.get(i as u32).unwrap(),
            Error::DepositMismatch,
        );
        desired.push_back(available);
    }
    let deposited = deposit_amounts(env, &m.destination.address, &desired, &draft.min_deposit);
    for (i, asset) in m.destination_assets.iter().enumerate() {
        authorize_transfer(
            env,
            &asset.asset,
            &m.destination.address,
            deposited.get(i as u32).unwrap(),
        );
    }
    let _: Val = env.invoke_contract(
        &m.destination.address,
        &symbol_short!("deposit"),
        (
            desired.clone(),
            draft.min_deposit.clone(),
            me.clone(),
            false,
        )
            .into_val(env),
    );
    for (i, asset) in m.destination_assets.iter().enumerate() {
        let index = tokens.first_index_of(&asset.asset).unwrap();
        ensure(
            env,
            balance(env, &asset.asset, &me)
                == add(
                    env,
                    gb.get(index).unwrap(),
                    sub(
                        env,
                        desired.get(i as u32).unwrap(),
                        deposited.get(i as u32).unwrap(),
                    ),
                ),
            Error::DepositMismatch,
        );
    }
    let shares = token::Client::new(env, &m.destination.address);
    let minted = sub(env, shares.balance(&me), dg0);
    ensure(
        env,
        minted >= draft.min_destination_shares,
        Error::OutcomeBelowMinimum,
    );
    shares.transfer(&me, &draft.exit.owner, &minted);
    ensure(
        env,
        shares.balance(&me) == dg0 && shares.balance(&draft.exit.owner) == add(env, do0, minted),
        Error::UnexpectedBalance,
    );
    let mut refunds = Vec::new(env);
    for (i, asset) in tokens.iter().enumerate() {
        let refund = sub(env, balance(env, &asset, &me), gb.get(i as u32).unwrap());
        ensure(env, refund >= 0, Error::UnexpectedBalance);
        if refund > 0 {
            token::Client::new(env, &asset).transfer(&me, &draft.exit.owner, &refund);
        }
        ensure(
            env,
            balance(env, &asset, &me) == gb.get(i as u32).unwrap()
                && balance(env, &asset, &draft.exit.owner)
                    == add(env, ob.get(i as u32).unwrap(), refund),
            Error::UnexpectedBalance,
        );
        refunds.push_back(refund);
    }
    (minted, deposited, converted, refunds)
}

fn maintain(env: &Env) {
    // Five second cadence is a TTL target, never a deadline oracle.
    let max = env.storage().max_ttl();
    env.storage()
        .instance()
        .extend_ttl(core::cmp::min(120_960, max), core::cmp::min(518_400, max));
}

#[contract]
pub struct Migration;
#[contractimpl]
impl Migration {
    pub fn __constructor(env: Env, profile: MigrationProfile) {
        migration_dependencies(&env, &profile);
        let digest: BytesN<32> = env.crypto().sha256(&profile.clone().to_xdr(&env)).into();
        env.storage().instance().set(&Key::Profile, &profile);
        env.storage().instance().set(&Key::Digest, &digest);
        maintain(&env);
    }
    pub fn profile(env: Env) -> MigrationProfile {
        env.storage().instance().get(&Key::Profile).unwrap()
    }
    pub fn profile_hash(env: Env) -> BytesN<32> {
        env.storage().instance().get(&Key::Digest).unwrap()
    }
    pub fn extend_life(env: Env) {
        maintain(&env);
    }

    pub fn execute_migration(env: Env, migration: MigrationIntent) -> MigrationReceipt {
        let intent = migration.exit.clone();
        let mp = Self::profile(env.clone());
        let p = mp.source.clone();
        ensure(
            &env,
            migration.convert_inputs.len() == mp.destination_assets.len()
                && migration.min_deposit.len() == mp.destination_assets.len()
                && migration.min_destination_shares > 0,
            Error::BadSchema,
        );
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
        migration_dependencies(&env, &mp);
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
        for a in mp.destination_assets.iter() {
            if !tokens.contains(&a.asset) {
                tokens.push_back(a.asset.clone());
                gb.push_back(balance(&env, &a.asset, &me));
                ob.push_back(balance(&env, &a.asset, &intent.owner));
            }
        }
        let destination_guard0 = balance(&env, &mp.destination.address, &me);
        let destination_owner0 = balance(&env, &mp.destination.address, &intent.owner);
        let out0 = balance(&env, &p.settlement, &me);
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
        let (destination_shares, deposited, converted, refunds) = deposit_and_refund(
            &env,
            &mp,
            &migration,
            &tokens,
            &gb,
            &ob,
            destination_guard0,
            destination_owner0,
            received,
        );
        ensure(
            &env,
            shares.balance(&intent.owner) == sub(&env, s0, intent.shares)
                && shares.balance(&me) == g0,
            Error::UnexpectedBalance,
        );
        migration_dependencies(&env, &mp);
        maintain(&env);
        let receipt = MigrationReceipt {
            destination: mp.destination.address,
            destination_shares,
            deposited,
            converted,
            refunds,
            refund_assets: tokens,
            intent_id: intent.intent_id.clone(),
            intent_hash: env.crypto().sha256(&migration.clone().to_xdr(&env)).into(),
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
            .publish((Symbol::new(&env, "migrate_v1"),), &receipt);
        receipt
    }
}
