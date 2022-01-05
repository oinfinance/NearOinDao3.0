use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{Base64VecU8, U128, U64};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{AccountId, Balance, BorshStorageKey, Gas, PublicKey};
use std::cmp::Eq;
use std::hash::Hash;
use uint::construct_uint;

pub const ST_NEAR: &str = "meta-v2.pool.testnet";
pub const ST_OIN: &str = "a61175c3dd4bee8a854ffc27c41e39e8e8161d11.factory.goerli.testnet";
// pub const ST_NEAR: &str = "meta-pool.near";
// pub const ST_OIN: &str = "9aeb50f542050172359a0e1a25a9933bc8c01259.factory.bridge.near";
pub const DATA_IMAGE_SVG_ST_USD_ICON: &str = "data:image/png;base64,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";
pub const ERR_PARSE: &str = "fail to parse";

pub const ERR_MUL: &str = "ERR_MUL_OVERFLOW";
pub const ERR_DIV: &str = "ERR_DIV_INSUFFICIENT";
pub const ERR_SUB: &str = "ERR_SUB_INSUFFICIENT";
pub const ERR_ADD: &str = "ERR_ADD_INSUFFICIENT";
pub const ERR_NOT_REGISTER: &str = "Not register in this contract";
pub const ERR_NO_COIN: &str = "NO THE REWARD COIN";
pub const ERR_NOT_REGISTER_REWARD: &str = "Not register the reward coin with account.";
pub const ERR_REWARD_ZERO: &str = "The reward of account is zero.";
pub const SYSTEM_PAUSE: &str = "The current function has been suspended.";
/* Gas */
pub const GAS_FOR_UPGRADE_CALL: Gas = 50_000_000_000_000;

pub const GAS_FOR_DEPLOY_CALL: Gas = 5_000_000_000_000;

pub const GAS_FOR_FT_TRANSFER_CALL: Gas = 20_000_000_000_000;

pub const ONE_YOCTO_NEAR: Balance = 1;

/* Token */
pub const ONE_TOKEN: Balance = 1_000_000_000_000_000_000_000_000;
pub const ONE_COIN: Balance = 100_000_000;

// TOKEN BIT - COST BIT 16bit...
pub const STAKE_RATIO_BASE: Balance = ONE_TOKEN / ONE_COIN;
/* params */

// A total value of 180 is scheduled to be liquidated
pub const INIT_LIQUIDATION_LINE: Balance = 160_000_000;
pub const MIN_LIQUIDATION_LINE: Balance = 150_000_000;
pub const INIT_GAS_RATIO_LINE: Balance = 500_000;
pub const INIT_ALLOT_RATIO_LINE: Balance = 108_000_000;
pub const INIT_LIQUIDATION_FEE_LINE: Balance = 2_000_000;
pub const INIT_NO_LIQUIDATION_FEE_RATE: Balance =
    INIT_ALLOT_RATIO_LINE + INIT_GAS_RATIO_LINE + INIT_LIQUIDATION_FEE_LINE;

pub const INIT_MIN_RATIO_LINE: Balance = 110_000_000;
pub const INIT_STABLE_FEE_RATE: Balance = 200_000_000_000_000;
pub const MAX_STABLE_FEE_RATE: Balance = 5_000_000_000_000_000;

pub const INIT_COIN_UPPER_LIMIT: Balance = 3_000_000_000_000_000;
pub const MAX_COIN_UPPER_LIMIT: Balance = 1_000_000_000_000_000_000;

pub const MIN_ALLOT_RATIO: Balance = 100_000_000;
pub const MAX_ALLOT_RATIO: Balance = 200_000_000;
pub const MAX_GAS_COMPENSATION_RATIO: Balance = 1_000_000;
pub const MAX_LIQUIDATION_FEE_RATIO: Balance = 2_000_000;

// 36 bit 24+8 = 32
pub const INIT_INDEX: Balance = 100_000_000_000_000_000_000_000_000_000_000;
pub const INIT_STABLE_INDEX: Balance = ONE_TOKEN / ONE_COIN;

/* esm */
pub const INIT_LIVE: u8 = 1;

/* lib */
pub const INIT_TOKEN_PRICE: Balance = 0;
pub const INIT_ORACLE_TIME: u64 = 0;
pub const POKE_INTERVAL_TIME: u64 = 172_800_000_000_000;
pub const INIT_TOTAL_COIN: u128 = 0;
pub const INIT_TOTAL_TOKEN: u128 = 0;

// near It's one block per second
pub const BLOCK_PER_YEAR: u128 = 31536000;
pub const NANO_CONVERSION: u64 = 1_000_000_000;
pub const REWARD_UPPER_BOUND: u64 = 20;

//Maximum amount of pledge deposit CASH_PLEDGE
pub const INIT_GUARANTEE_LIMIT: Balance = 1_000_000_000;
pub const INIT_TOTAL_GUARANTEE: u128 = 0;
pub const INIT_MIN_MINT_AMOUNT: Balance = 10_000_000_000; //_000_000_000_000_000;

/**stablepool */
pub const ERR_GET: &str = "Not Legal get this contract";

//every second give 0.0001 near mint reward
// pub const STABLE_SPEED: u128 = 0;
pub const STABLE_SPEED: u128 = 1_000_000;
pub const SCALE_FACTOR: u128 = 1_000_000_000_000;
pub const DECIMAL_PRECISION: u128 = 1_000_000_000_000_000_000_000_000; // 24
pub(crate) type InnerU256 = [u64; 4];

/**for multisign */
pub const DEFAULT_NUM_CONFIRM_RATIO: u64 = 60;
pub const DEFAULT_REQUEST_COOLDOWN: u64 = 12*60*60_000_000_000; //12h
pub const MAX_REQUEST_COOLDOWN: u64 = 604_800_000_000_000; //1 week
pub const REQUEST_EXPIRE_TIME: u64 = 3 * 86_400_000_000_000; //one_day
pub const ONE_FOR_NUM_RATIO: u64 = 100; //100% Pass rate

pub type RequestId = u32;

/// Lowest level action that can be performed by the multisig contract.
#[derive(Clone, PartialEq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(tag = "type", crate = "near_sdk::serde")]
pub enum MultiSigRequestAction {
    /// Call function on behalf of this contract.
    FunctionCall {
        method_name: String,
        args: Base64VecU8,
        deposit: U128,
        gas: U64,
    },
}
// The request the user makes specifying the receiving account and actions they want to execute (1 tx)
#[derive(Clone, PartialEq, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct MultiSigRequest {
    pub actions: Vec<MultiSigRequestAction>,
}

// An internal request wrapped with the signer_pk and added timestamp to determine num_requests_pk and prevent against malicious key holder gas attacks
#[derive(Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct MultiSigRequestWithSigner {
    pub request: MultiSigRequest,
    pub signer_pk: PublicKey,
    pub added_timestamp: u64,
    pub confirmed_timestamp: u64,
    pub is_executed: bool,
    pub cool_down: u64,
    pub num_confirm_ratio: u64,
    pub mul_white_num: u64,
}

/**for stablepool */
#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    UnsortedVaults,
    Deposits,
    SPDisReward,
    SPMinReward,
    EpochSum { epoch: u128 },
    DepositSnapshots,
    EpochG,
    ScaleSums,
    ScaleG { epoch: u128 },
    Requests,
    Confirmations,
    NumRequestsPk,
    MulWhiteList,
    WhiteList,
    AccountCoin,
    AccountToken,
    AccountAllot,
    RewardCoins,
    AccountReward,
    AccountStable,
    Guarantee,
}

#[derive(Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct Snapshots {
    pub s: InnerU256, /* for dis */
    pub p: u128,      /* for offset */
    pub g: InnerU256, /* for mint */
    pub scale: u64,
    pub epoch: u128,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct ScaleSum {
    pub scale: u128, /* scale for P */
    pub sum: u128,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct SystemG {
    pub g_block_num: u64, /* current time */
    pub g_system: InnerU256,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct RewardCoin {
    pub token: AccountId,      //Currency contract address
    pub total_reward: Balance, //The total reward
    pub reward_speed: u128,    //Reward rate
    pub index: u128,           /* Up to 36 bits will do... */
    pub double_scale: u128,    /* init_index ->The initial value... */
    pub block_number: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct UserDebt {
    pub stake_token_num: Balance,
    pub mint_usdo_num: Balance,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Hash, Clone, Copy, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct UserReward {
    //Record unclaimed rewards and reward coefficients of individual pledge pool
    pub reward: Balance,
    pub index: Balance,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Hash, Clone, Copy, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct UserStable {
    pub unpaid_stable_fee: Balance, //Record personal unpaid stabilization fees
    pub index: Balance,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Hash, Clone, Copy, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct AccountAllot {
    pub account_allot_debt: Balance,
    pub account_allot_token: Balance,
}

construct_uint! {
    pub struct U256(4);
}
