use soroban_sdk::{contracterror, contracttype, Address, BytesN, String, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodePin {
    pub address: Address,
    pub wasm_hash: BytesN<32>,
}
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetRoute {
    pub asset: Address,
    pub pair: Vec<CodePin>,
}
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteProfile {
    pub schema: u32,
    pub network_id: BytesN<32>,
    pub vault: CodePin,
    pub router: CodePin,
    pub factory: CodePin,
    pub settlement: Address,
    pub assets: Vec<AssetRoute>,
    pub max_seconds: u64,
    pub max_ledgers: u32,
}
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExitIntent {
    pub schema: u32,
    pub network_id: BytesN<32>,
    pub guard: Address,
    pub profile_hash: BytesN<32>,
    pub intent_id: BytesN<32>,
    pub owner: Address,
    pub recipient: Address,
    pub shares: i128,
    pub min_withdraw: Vec<i128>,
    pub min_swap: Vec<i128>,
    pub min_receive: i128,
    pub valid_before_time: u64,
    pub valid_before_ledger: u32,
}
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationReceipt {
    pub destination: Address,
    pub destination_shares: i128,
    pub deposited: Vec<i128>,
    pub converted: Vec<i128>,
    pub refund_assets: Vec<Address>,
    pub refunds: Vec<i128>,
    pub intent_id: BytesN<32>,
    pub intent_hash: BytesN<32>,
    pub profile_hash: BytesN<32>,
    pub owner: Address,
    pub shares_spent: i128,
    pub received: i128,
    pub withdrawn: Vec<i128>,
    pub swapped_out: Vec<i128>,
    pub ledger: u32,
    pub timestamp: u64,
}
#[contracttype]
#[derive(Clone)]
pub enum Key {
    Profile,
    Digest,
    Used(Address, BytesN<32>),
}

// Matches the pinned DeFindex common crate's contract ABI.
#[contracttype]
#[derive(Clone)]
pub struct Strategy {
    pub address: Address,
    pub name: String,
    pub paused: bool,
}
#[contracttype]
#[derive(Clone)]
pub struct AssetStrategySet {
    pub address: Address,
    pub strategies: Vec<Strategy>,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    BadSchema = 1001,
    WrongDomain = 1002,
    Expired = 1003,
    DeadlineTooFar = 1004,
    BadAmount = 1005,
    UnsupportedRecipient = 1006,
    DependencyChanged = 1007,
    AlreadyExecuted = 1008,
    InsufficientShares = 1009,
    WithdrawalMismatch = 1010,
    SwapMismatch = 1011,
    OutcomeBelowMinimum = 1012,
    UnexpectedBalance = 1013,
    Arithmetic = 1014,
    UnsupportedIdentity = 1015,
    DepositMismatch = 1016,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationProfile {
    pub source: RouteProfile,
    pub destination: CodePin,
    pub destination_assets: Vec<AssetRoute>,
}
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationIntent {
    pub exit: ExitIntent,
    pub convert_inputs: Vec<i128>,
    pub min_deposit: Vec<i128>,
    pub min_destination_shares: i128,
}
