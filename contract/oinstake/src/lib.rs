use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{Base64VecU8, ValidAccountId, U128};
use near_sdk::{
    env, ext_contract, log, near_bindgen, serde_json, AccountId, Balance,
    PanicOnDefault, Promise, PromiseOrValue, PromiseResult, PublicKey,
};

use near_sdk::serde::{Deserialize, Serialize};

use near_sdk::collections::*;
use std::collections::*;
use std::convert::TryInto;

use near_contract_standards::fungible_token::core_impl::ext_fungible_token;
use near_contract_standards::fungible_token::FungibleToken;

mod allot;
mod dparam;
mod esm;
mod ft;
mod functional;
mod multisign;
mod oracle;
mod pool;
mod reward;
mod sortvault;
mod stablefee;
mod stablepool;
mod types;
mod upgrade;
mod usdo;
mod views;
mod whitelist;

use types::*;

near_sdk::setup_alloc!();

// TODO Ok

#[ext_contract(ext_self)]
pub trait SelfCallbacks {
    fn after_withdraw_token(&mut self, account_id: AccountId, amount: Balance, guarantee: Balance);
    fn on_mint_transfer(&mut self, holder: AccountId, amount: U128);
    fn storage_deposit(&mut self, account_id: ValidAccountId, registration_only: Option<bool>);
    fn ft_transfer(&mut self, receiver_id: ValidAccountId, amount: U128, msg: Option<String>);
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct OinStake {
    owner_id: AccountId,

    whitelist: UnorderedSet<AccountId>,
    min_mint_amount: Balance,

    total_coin: Balance,
    total_token: Balance,
    account_coin: LookupMap<AccountId, Balance>,
    account_token: LookupMap<AccountId, Balance>,

    total_guarantee: Balance,
    guarantee: LookupMap<AccountId, Balance>,

    //Pledge pool redistribution scenario allot--> redistribution
    sys_allot_debt: Balance,
    sys_allot_token: Balance,
    account_allot: LookupMap<AccountId, AccountAllot>,
    token_offset: Balance,
    debt_offset: Balance,
    gas_compensation_ratio: Balance,//The gas compensation ratio defaults to 0.5%
    liquidation_fee_ratio: Balance,//The default clearing commission rate is 2%
    allot_ratio: Balance,//The reallocation ratio defaults to 108%
    total_liquidation_fee: Balance, //Statistical clearing commission

    //Pledge pool reward currency information up to 20 coins
    reward_coins: UnorderedMap<AccountId, RewardCoin>,
    account_reward: LookupMap<String /* account + reward_coin */, UserReward>,

    /* params */
    liquidation_line: u128,
    coin_upper_limit: u128,//Maximum amount of system STUSD generated
    guarantee_limit: u128,//The minimum deposit

    /* esm */
    is_deposit_live : u8,//Storage status 0 Off 1 on
    is_withdraw_live: u8,//State 0 Off 1 on
    is_mint_coin_live: u8,//Generates state 0 off 1 on
    is_burn_coin_live: u8,//Redemption status 0 Off 1 on
    is_claim_reward_live: u8,//Pledge pool reward receiving status 0 off 1 on
    is_liquidate_live: u8,//Liquidation status of pledge pool 0 off 1 on
    is_provide_to_sp_live: u8,//Stable pool storage status 0 Off 1 On
    is_withdraw_from_sp_live : u8,//Stability pool status 0 Off 1 On
    is_claim_token_live : u8,//Stable pool reward status 0 off 1 on

    /* oracle */
    token_price: u128,
    token_poke_time: u64,
    oracle: AccountId,

    /* extend */
    stake_storage_usage: u128,

    /* usdo */
    token: FungibleToken,
    name: String,
    symbol: String,
    reference: String,
    reference_hash: Base64VecU8,
    decimals: u8,

    /* multisign */
    request_cooldown: u64,
    num_confirm_ratio: u64,
    request_nonce: RequestId,//todo view add
    requests: UnorderedMap<RequestId, MultiSigRequestWithSigner>,
    confirmations: UnorderedMap<RequestId, HashSet<PublicKey>>,
    num_requests_pk: UnorderedMap<PublicKey, u32>,
    mul_white_list: UnorderedSet<AccountId>,
    // per key

    /* stable fee */
    total_unpaid_stable_fee: Balance,
    total_paid_stable_fee: Balance,
    stable_block_number: u64,
    stable_index: Balance,
    stable_fee_rate: u128,
    
    black_hole: AccountId,
    stake_token: AccountId,
    stable_token: AccountId,
    account_stable: LookupMap<String, UserStable>,
    total_stable_fee_transfer_failed: Balance,

    /* stablepool */
    total_usdo_deposits: u128,
    deposits: LookupMap<AccountId, Balance>, // depositor address -> Deposit struct
    deposit_snapshots: LookupMap<AccountId, Snapshots>, // depositor address -> snapshots struct
    min_reward: LookupMap<AccountId, Balance>, // depositor address -> min_rewardlast
    dis_reward: LookupMap<AccountId, Balance>, // depositor address -> dis_reward_rewardlast

    /*The 'S' sums are stored in a nested mapping (epoch => scale => sum):*/
    epoch_to_scale_to_sum: LookupMap<u128, LookupMap<u64, InnerU256>>, // depositor address -> snapshots struct
    epoch_to_scale_to_g: LookupMap<u128, LookupMap<u64, SystemG>>, // depositor address -> snapshots struct
    unsorted_vaults: Vec<ValidAccountId>,                          //
    account_to_index: LookupMap<ValidAccountId, usize>,            // Get index

    p_system: u128,//P of the system
    current_scale: u64, //The current scale Each time the scale of P shifts by SCALE_FACTOR, the scale is incremented by 1
    current_epoch: u128, //The current epoch  With each offset that fully empties the Pool, the epoch is incremented by 1
    last_near_error_offset: u128, // Error offset of STnear
    last_usdo_loss_error_offset: u128,//Error offset of STUSD
    total_token_sp: u128,  //Near amount of stable pool redistribution
    reward_sp: u128,       //Total stable pool rewards
    reward_speed_sp: u128, //Steady pool bonus speed
}

#[near_bindgen]
impl OinStake {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        let mut this = Self {
            owner_id: owner_id.clone(),

            whitelist: UnorderedSet::new(StorageKey::WhiteList),

            min_mint_amount: INIT_MIN_MINT_AMOUNT,

            total_coin: INIT_TOTAL_COIN,
            total_token: INIT_TOTAL_TOKEN,
            account_coin: LookupMap::new(StorageKey::AccountCoin),
            account_token: LookupMap::new(StorageKey::AccountToken),
            total_guarantee: INIT_TOTAL_GUARANTEE,
            guarantee: LookupMap::new(StorageKey::Guarantee), //押金

            //Debt allot
            sys_allot_debt: 0,
            sys_allot_token: 0,
            account_allot: LookupMap::new(StorageKey::AccountAllot),
            token_offset: 0,
            debt_offset: 0,
            gas_compensation_ratio: INIT_GAS_RATIO_LINE,
            liquidation_fee_ratio: INIT_LIQUIDATION_FEE_LINE,//The default clearing commission rate is 2%
            allot_ratio: INIT_ALLOT_RATIO_LINE,
            total_liquidation_fee: 0,

            //The cost of storing the reward token and whitelist is provided by the project side
            reward_coins: UnorderedMap::new(StorageKey::RewardCoins),

            // The token deposit for the user is not retrievable and is only injected at registration time
            account_reward: LookupMap::new(StorageKey::AccountReward),

            // The rest of the cost is provided by the project side
            /* params */
            liquidation_line: INIT_LIQUIDATION_LINE,
            coin_upper_limit: INIT_COIN_UPPER_LIMIT,
            guarantee_limit: INIT_GUARANTEE_LIMIT, //The deposit amount

            /* esm */
            is_deposit_live : INIT_LIVE,
            is_withdraw_live: INIT_LIVE,
            is_mint_coin_live: INIT_LIVE,
            is_burn_coin_live: INIT_LIVE,
            is_claim_reward_live: INIT_LIVE,
            is_liquidate_live: INIT_LIVE,
            is_provide_to_sp_live: INIT_LIVE,
            is_withdraw_from_sp_live : INIT_LIVE,
            is_claim_token_live : INIT_LIVE,

            /* oracle */
            token_price: INIT_TOKEN_PRICE,
            token_poke_time: INIT_ORACLE_TIME,
            oracle: owner_id.clone(),

            stake_storage_usage: 0,

            /* usdo */
            token: FungibleToken::new(b"stusd".to_vec()),
            name: "nUSDO".to_string(),
            symbol: "nUSDO".to_string(),
            decimals: 8,
            reference: String::default(),
            reference_hash: Base64VecU8(vec![]),

            /* multisign */
            request_cooldown: DEFAULT_REQUEST_COOLDOWN,
            num_confirm_ratio: DEFAULT_NUM_CONFIRM_RATIO,
            request_nonce: 0,
            mul_white_list: UnorderedSet::new(StorageKey::MulWhiteList),
            requests: UnorderedMap::new(StorageKey::Requests),
            confirmations: UnorderedMap::new(StorageKey::Confirmations),
            num_requests_pk: UnorderedMap::new(StorageKey::NumRequestsPk),

            /* stable */
            total_unpaid_stable_fee: 0,
            total_paid_stable_fee: 0,
            stable_block_number: 0,
            stable_index: INIT_STABLE_INDEX,
            stable_fee_rate: INIT_STABLE_FEE_RATE, /* 16bit */
            black_hole: "0".repeat(64),
            stake_token: ST_NEAR.to_string(),
            stable_token: ST_OIN.to_string(),
            account_stable: LookupMap::new(StorageKey::AccountStable),
            total_stable_fee_transfer_failed: 0,

            /*for stablepool*/
            p_system: DECIMAL_PRECISION,
            total_token_sp: 0,
            reward_sp: 0,
            reward_speed_sp: STABLE_SPEED,
            current_scale: 0,
            current_epoch: 0,
            total_usdo_deposits: 0,

            deposits: LookupMap::new(StorageKey::Deposits),
            min_reward: LookupMap::new(StorageKey::SPMinReward), 
            dis_reward: LookupMap::new(StorageKey::SPDisReward), // depositor address -> snapshots struct
            deposit_snapshots: LookupMap::new(StorageKey::DepositSnapshots), // depositor address -> reward
            epoch_to_scale_to_sum: LookupMap::new(StorageKey::DepositSnapshots),
            epoch_to_scale_to_g: LookupMap::new(StorageKey::EpochG),
            unsorted_vaults: Vec::new(), //for list of liquitation
            account_to_index: LookupMap::new(StorageKey::UnsortedVaults),

            last_near_error_offset: 0, // Error trackers for the error correction in the offset calculation
            last_usdo_loss_error_offset: 0,
        };

        // TODO Ok-> This part of the call is called by the contract itself
        this.whitelist.insert(&env::current_account_id());
        this.whitelist.insert(&owner_id);
        this.init_reward_coin();
        this.measure_stake_storage_usage();

        /*init epoch map*/
        let mut scale_to_sum = LookupMap::new(StorageKey::ScaleSums);
        let mut scale_to_g = LookupMap::new(StorageKey::ScaleG {
            epoch: this.current_epoch.clone(),
        });

        /*init S to 0*/
        scale_to_sum.insert(&0, &U256::zero().0);
        scale_to_sum.insert(&1, &U256::zero().0);
        this.epoch_to_scale_to_sum
            .insert(&this.current_epoch, &scale_to_sum);
        scale_to_g.insert(
            &this.current_scale,
            &SystemG {
                g_system: U256::zero().0,
                g_block_num: 0,
            },
        );
        /*init G to 0,consider way to init from the beginning*/
        this.epoch_to_scale_to_g
            .insert(&this.current_epoch, &scale_to_g);

        this
    }

    // TODO [OK] Measure the storage bytes required for the current pledge, and the cost needs to be multiplied by env::sotrage_usage()
    pub(crate) fn measure_stake_storage_usage(&mut self) {
        self.stake_storage_usage = self.internal_measure_debt() + self.internal_measure_reward();
    }
    pub(crate) fn internal_measure_debt(&mut self) -> u128 {
        let prev_storage = env::storage_usage();
        let tmp_account_id = "a".repeat(64);

        self.account_coin.insert(&tmp_account_id, &0u128);
        self.account_token.insert(&tmp_account_id, &0u128);
        self.guarantee.insert(&tmp_account_id, &0u128);


        self.dis_reward.insert(&tmp_account_id, &0u128);
        self.min_reward.insert(&tmp_account_id, &0u128);
        self.deposits.insert(&tmp_account_id, &0u128);
        self.deposit_snapshots.insert(&tmp_account_id, &Snapshots {
            scale: 0u64,
            epoch: 0u128,
            p: 0u128,
            g: U256::zero().0,
            s: U256::zero().0,
        });

        self.account_stable.insert(
            &tmp_account_id,
            &UserStable {
                unpaid_stable_fee: 0u128,
                index: 0u128,
            },
        );

        let value = (env::storage_usage() - prev_storage) as u128;
        self.account_coin.remove(&tmp_account_id);
        self.account_token.remove(&tmp_account_id);
        self.account_stable.remove(&tmp_account_id);
        self.guarantee.remove(&tmp_account_id);
        value
    }
    pub(crate) fn internal_measure_reward(&mut self) -> u128 {
        let prev_storage = env::storage_usage();
        let tmp_account_id = "a".repeat(64);

        self.account_reward.insert(
            &tmp_account_id,
            &UserReward {
                index: 0u128,
                reward: 0u128,
            },
        );
        let value = ((env::storage_usage() - prev_storage) as u128) * 20;
        self.account_reward.remove(&tmp_account_id);
        value
    }

    #[payable]
    pub(crate) fn deposit_token(&mut self, _amount: u128, _sender_id: ValidAccountId) {

        assert!(self.is_deposit_paused(), "{}", SYSTEM_PAUSE);
        self.assert_is_poked();

        let sender_id = AccountId::from(_sender_id);
        assert!(_amount > 0, "Deposit token amount must greater than zero.");

        if let Some(personal_token) = self.account_token.get(&sender_id) {

            //Determine the user's first pledge
            if let Some(0) = self.guarantee.get(&sender_id) {
                assert!(
                    _amount >= self._min_amount_token(),
                    "Deposit token amount must greater the minimum deposit token."
                );
                self.guarantee.insert(&sender_id.clone(), &self.guarantee_limit);
                self.total_guarantee = self.total_guarantee.checked_add(self.guarantee_limit).expect(ERR_ADD);
                //Initialize the linked list at the first pledge
                self._vault_init(sender_id.clone().try_into().unwrap());
            }

            self.update_personal_data(sender_id.clone());

            let result = personal_token.checked_add(_amount).expect(ERR_ADD);

            self.account_token.insert(&sender_id.clone(), &result);
            self.total_token = self.total_token.checked_add(_amount).expect(ERR_ADD);
        } else {
            env::panic(b"Not regist.");
        }
        log!("Deposit token amount:{}", _amount);
    }

    #[payable]
    //Obtain dynamic maximum withdrawal when withdrawing A Deposite (real-time calculation of stability fee is added)
    pub fn withdraw_token(&mut self, amount: U128) {

        assert!(self.is_withdraw_paused(), "{}", SYSTEM_PAUSE);
        let mut amount = amount.0;
        let account_id = env::predecessor_account_id();

        let avaliable_token = self.internal_avaliable_token(account_id.clone());
        log!("token :{} amount: {}", avaliable_token, amount);
        assert!(avaliable_token >= amount, "Insufficient avaliable token.");

        let (debt, _ , guarantee) = self.get_debt(account_id.clone());
       
        //User free of debt
        if debt == guarantee {
            if avaliable_token.checked_sub(amount).expect(ERR_SUB) < self._min_amount_token() {
                amount = avaliable_token;
            }
        }

        //The three updates are processed together
        self.update_personal_data(account_id.clone());
        
        let account_token = self.account_token.get(&account_id.clone()).expect(ERR_NOT_REGISTER);

        self.total_token = self.total_token.checked_sub(amount).expect(ERR_SUB);

        self.account_token.insert(
            &account_id.clone(),
            &(account_token
                .checked_sub(amount).expect(ERR_SUB)),
        );

        if let Some(0) = self.account_token.get(&account_id.clone()) {
            self._vault_remove(account_id.clone().try_into().unwrap());
            self.total_guarantee = self
                .total_guarantee
                .checked_sub(self.guarantee.get(&account_id.clone()).expect(ERR_NOT_REGISTER))
                .expect(ERR_SUB);
            self.guarantee.insert(&account_id.clone(), &0u128);
        }

        ext_self::ft_transfer(
            account_id.clone().try_into().unwrap(),
            U128(amount.into()),
            None,
            &ST_NEAR,
            ONE_YOCTO_NEAR,
            GAS_FOR_FT_TRANSFER_CALL,
        )
        .then(ext_self::after_withdraw_token(
            account_id.clone(),
            amount.into(),
            guarantee.into(),
            &env::current_account_id(),
            0,
            GAS_FOR_FT_TRANSFER_CALL,
        ));
    }

    /*
        TODO [OK] Calculate personal reward, personal stability fee, personal debt pledge allocation and update
        In this order, we need to calculate the stability fee update first and then calculate the allocation and update
        Users need to update each operation, because the stability fee is calculated in coins, and the allocated debt will be added to coins
    */
    pub(crate) fn update_personal_data(&mut self, account_id: AccountId) {
        self.update_personal_token(account_id.clone());
        self.update_stable_fee(account_id.clone());
        self.update_account_allot(account_id.clone());
    }

    //TODO [OK] Minimum pledge Minimum amount pledged
    pub(crate) fn _min_amount_token(&self) -> u128 {
        (U256::from(self.min_mint_amount)
            .checked_add(U256::from(self.guarantee_limit)).expect(ERR_ADD)
        )
        .checked_mul(U256::from(self.liquidation_line)).expect(ERR_MUL)
        .checked_mul(U256::from(INIT_STABLE_INDEX)).expect(ERR_MUL)
        .checked_div(U256::from(self.token_price)).expect(ERR_DIV)
        .as_u128()
    }

    // TODO Ok -> System pledge rate
    pub(crate) fn internal_sys_ratio(&self) -> u128 {
        self.assert_is_poked();
        let token_usd = U256::from(self.total_token)
            .checked_mul(U256::from(self.token_price))
            .expect(ERR_MUL); /* 32 */

        if token_usd == U256::from(0) {
            0
        } else {

            let current_block_number = self.to_nano(env::block_timestamp());
            //Total stabilization fee is not paid
            let total_unpaid_stable_fee = self.total_unpaid_stable_fee
                .checked_add(
                    self.stable_fee_rate//16
                        .checked_mul(
                            (current_block_number as u128)
                                .checked_sub(self.stable_block_number as u128).expect(ERR_SUB)
                            ).expect(ERR_MUL)
                        .checked_mul(self.total_coin).expect(ERR_MUL)//8
                        .checked_div(BLOCK_PER_YEAR).expect(ERR_DIV)
                        .checked_div(INIT_STABLE_INDEX).expect(ERR_DIV)//16
                        
                ).expect(ERR_ADD);
           
            let total_coin = self.total_coin
                .checked_add(self.total_guarantee).expect(ERR_ADD)
                .checked_add(total_unpaid_stable_fee).expect(ERR_ADD);
        
            token_usd
                .checked_div(U256::from(STAKE_RATIO_BASE))
                .expect(ERR_DIV)
                .checked_div(U256::from(total_coin))
                .expect(ERR_DIV)
                .as_u128()
        }
    }

    // TODO Ok -> Personal pledge rate
    pub(crate) fn internal_user_ratio(&self, account: AccountId) -> u128 {
        self.assert_is_poked();
        //let token = self.account_token.get(&account.clone()).expect(ERR_NOT_REGISTER);
        let (debt, token, _ ) = self.get_debt(account.clone());
        if token == 0 {
            0
        } else {

            //Get the amount of outstanding stabilization fee
            let unpaid_stable_fee = self.internal_user_unpaid_stable(account.clone());

            let total_coin = debt.checked_add(unpaid_stable_fee).expect(ERR_ADD);

            let token_usd = U256::from(token)
                .checked_mul(U256::from(self.token_price))
                .expect(ERR_MUL);

            token_usd
                .checked_div(U256::from(total_coin))
                .expect(ERR_DIV)
                .checked_div(U256::from(STAKE_RATIO_BASE))
                .expect(ERR_DIV)
                .as_u128()
        }
    }

    // TODO [OK] -> Withdrawable TOKEN
    pub(crate) fn internal_avaliable_token(&self, account: AccountId) -> u128 {
        self.assert_is_poked();

        //Redo the current operation, adding a minimum amount of production judgment
        let (debt, token, guarantee) = self.get_debt(account.clone());
        //No allocation debt is generated to withdraw all tokens
        if debt == guarantee {
            token
        }else{
            //Minus guarantee obligations greater than the minimum production, all calculations shall be based on the minimum production
            let mut user_coin = debt.checked_sub(guarantee).expect(ERR_SUB);

            if  user_coin < self.min_mint_amount {
                user_coin = self.min_mint_amount;
            }
            //Get the amount of outstanding stabilization fee
            let unpaid_stable_fee = self.internal_user_unpaid_stable(account.clone());
            
            let coin_usd = (U256::from(user_coin)
                    .checked_add(U256::from(guarantee)).expect(ERR_ADD)
                    .checked_add(U256::from(unpaid_stable_fee)).expect(ERR_ADD)
                )
                .checked_mul(U256::from(self.liquidation_line))
                .expect(ERR_MUL)
                .checked_mul(U256::from(ONE_TOKEN))
                .expect(ERR_MUL); /* 40 */

            let token_usd = U256::from(token)
                .checked_mul(U256::from(self.token_price))
                .expect(ERR_MUL)
                .checked_mul(U256::from(ONE_COIN))
                .expect(ERR_MUL); /* 40 */

            if token_usd <= coin_usd {
                0
            }else{
                token_usd
                    .checked_sub(coin_usd)
                    .expect(ERR_SUB)
                    .checked_div(U256::from(self.token_price))
                    .expect(ERR_DIV)
                    .checked_div(U256::from(ONE_COIN))
                    .expect(ERR_DIV)
                    .as_u128() /* 24 */
            }
        }

    }

    // TODO [OK] -> Maximum number of coins generated
    pub(crate) fn internal_can_mint_amount(&self, account: AccountId) -> u128 {
        self.assert_is_poked();

        let (debt, token, _ ) = self.get_debt(account.clone());

        let unpaid_stable_fee = self.internal_user_unpaid_stable(account);

       // The current maximum amount of debt generated (coin)
       //(Personal pledge token+ Personal allot_token)* Token price/current clearing line default 180% -
       // Generated debt (coin) - allot_debt - Deposit default 10- stabilization fee
        
        let max_coin = U256::from(token)/*24 */
            .checked_mul(U256::from(self.token_price)).expect(ERR_MUL)/*8 */
            .checked_div(U256::from(self.liquidation_line)).expect(ERR_DIV)/*8 */
            .checked_div(U256::from(INIT_STABLE_INDEX)).expect(ERR_DIV)/*8 */
            .checked_sub(U256::from(debt)).unwrap_or(U256::from(0))
            .checked_sub(U256::from(unpaid_stable_fee)).unwrap_or(U256::from(0))
            .as_u128();

        max_coin
    }

    // TODO [OK] USDO A stabilization fee not paid by the user
    pub(crate) fn internal_user_unpaid_stable(&self, account: AccountId) -> u128 {

        let user_stable = self.account_stable.get(&account).expect(ERR_NOT_REGISTER);
        let coin = self.account_coin.get(&account).expect(ERR_NOT_REGISTER);
            
        let current_block_number = self.to_nano(env::block_timestamp());
   
        // Stability fee not paid by user + ((system index- personal index)* user coin) +
        //(current system time - system record last calculation time)* steady rate * user coin/ year seconds
        // User coin does not contain allot_coin;
        let _unpaid_stable_fee = user_stable.unpaid_stable_fee
            .checked_add(
                self.stable_index
                    .checked_sub(user_stable.index).expect(ERR_SUB)
                    .checked_mul(coin).expect(ERR_MUL)
                    .checked_div(INIT_STABLE_INDEX).expect(ERR_DIV)
            )
            .expect(ERR_ADD)
            .checked_add(
                self.stable_fee_rate//16
                    .checked_mul(
                        (current_block_number as u128)
                        .checked_sub(self.stable_block_number as u128).expect(ERR_SUB)
                        ).expect(ERR_MUL)
                .checked_mul(coin).expect(ERR_MUL)//8
                .checked_div(BLOCK_PER_YEAR).expect(ERR_DIV)
                .checked_div(INIT_STABLE_INDEX).expect(ERR_DIV)//16 
            )
            .expect(ERR_ADD);

        _unpaid_stable_fee
    }

    pub(crate) fn to_nano(&self ,timestamp: u64) -> u64 {
        timestamp.checked_div(NANO_CONVERSION).expect(ERR_DIV)
    }

    // TODO [OK] COINS
    #[payable]
    pub fn mint_coin(&mut self, amount: U128) {
        assert!(self.is_mint_coin_paused(), "{}", SYSTEM_PAUSE);

        let account_id = env::predecessor_account_id();
        let _amount = amount.into();

        assert!(
            self.total_coin.checked_add(_amount).expect(ERR_ADD) <= self.coin_upper_limit,
            "Exceeded coin amount"
        );
        
        self.update_personal_data(account_id.clone());

        let coin = self.account_coin.get(&account_id.clone()).expect(ERR_NOT_REGISTER);
        
        //Minimum production judgment
        let user_coin_total = coin.checked_add(_amount).expect(ERR_ADD);

        assert!(user_coin_total >= self.min_mint_amount, "Less than the minimum amount produced");
        
        //Maximum yield judgment
        let can_usdo = self.internal_can_mint_amount(account_id.clone());
        
        assert!(can_usdo >= _amount, "Insufficient amount");
        
        //Give env::current_account_id() mints and then transfer mints to env::predecessor_account_id(),
        self.internal_mint(env::current_account_id(), _amount);

        self.total_coin = self.total_coin.checked_add(_amount).expect(ERR_ADD);
        
        self.account_coin.insert(
            &account_id.clone(),
            &user_coin_total,
        );
        
        ext_fungible_token::ft_transfer(
            account_id.clone(),
            amount,
            None,
            &env::current_account_id(),
            ONE_YOCTO_NEAR,
            GAS_FOR_FT_TRANSFER_CALL,
        )
        .then(ext_self::on_mint_transfer(
            account_id.clone(),
            amount,
            &env::current_account_id(),
            0,
            GAS_FOR_FT_TRANSFER_CALL,
        ));
        
    }
    
    //TODO [OK] Destruction of debt When the debt is destroyed, both the debt and the stabilization fee are paid in stabilization currency. The stabilization fee is charged before the debt is destroyed
    #[payable]
    pub fn burn_coin(&mut self, amount: U128) {

        assert!(self.is_burn_coin_paused(), "{}", SYSTEM_PAUSE);
        self.assert_is_poked();

        let account_id = env::predecessor_account_id();
        let ft_balance_usd = self.ft_balance(account_id.clone());
        let mut _amount = amount.0;

        assert!(ft_balance_usd >= _amount, "Lack of balance");

        self.update_personal_data(account_id.clone());

        //Get outstanding stabilization fees
        let mut user_stable = self.account_stable.get(&account_id.clone()).expect(ERR_NOT_REGISTER);
        let unpaid_stable_fee  = user_stable.unpaid_stable_fee;
        
        //get debt
        let coin = self.account_coin.get(&account_id.clone()).expect(ERR_NOT_REGISTER);

        let unpaid_usd = coin.checked_add(unpaid_stable_fee).expect(ERR_ADD);

        if unpaid_usd < _amount {
            _amount = unpaid_usd;
        }

        //Collect the stabilization fee and then pay back the debt and determine whether the user has enough money to pay the stabilization fee
        if _amount > unpaid_stable_fee {
            //Amount of coin remaining to repay debt
            let payback_amount = _amount.checked_sub(unpaid_stable_fee).expect(ERR_SUB);
            
            let user_coin = coin.checked_sub(payback_amount).expect(ERR_SUB);

            if user_coin > 0 {
                //After the actual repayment of debt, it cannot be less than the minimum amount of production
                assert!( user_coin >= self.min_mint_amount, "Please return all coins");
            }

            self.total_coin = self.total_coin.checked_sub(payback_amount).expect(ERR_SUB);
            self.account_coin.insert(
                &account_id.clone(),
                &user_coin,
            );
            //Will return the debt burn operation
            self.internal_burn(account_id.clone(), payback_amount);
        }
        
        if unpaid_stable_fee > 0 {
            //Clearing the unpaid stability fee for users, reducing the total unpaid stability fee for the system, and accumulating the stability fee charged by the project party
            user_stable.unpaid_stable_fee = user_stable.unpaid_stable_fee
                .checked_sub(unpaid_stable_fee).expect(ERR_SUB);

            self.account_stable.insert(&account_id.clone(), &user_stable);
            //Total system unpaid stabilization fee
            self.total_unpaid_stable_fee = self.total_unpaid_stable_fee
                .checked_sub(unpaid_stable_fee).expect(ERR_SUB);
            
            //TODO A value for the stabilization fee charged by the project side is required
            self.total_paid_stable_fee =  self.total_paid_stable_fee.
                checked_add(unpaid_stable_fee).expect(ERR_ADD);

            //The stabilization fee is transferred to the owner
            self.token.internal_transfer(&account_id.clone(), &self.owner_id.clone(), unpaid_stable_fee.into(), None);
        } 
    }

    #[payable]
    pub fn liquidation(&mut self, account: AccountId) {
        assert!(self.is_liquidate_paused(), "{}", SYSTEM_PAUSE);
        
        //Check whether the user is registered
        let account_id = env::predecessor_account_id();
        assert!(self.is_register(account_id.clone()), "{}", ERR_NOT_REGISTER);

        let ratio = self.internal_user_ratio(account.clone());
        assert!(ratio > 0, "No current token");
        assert!(ratio < self.liquidation_line, "Not at the clearing line");

        self.update_personal_token(account.clone());
        self.update_stable_system_index();//Update the total amount of system unpaid stabilization fee

        let (allot_debt, allot_token, guarantee) = self.get_debt(account.clone());

        //The stabilization fee not paid by the liquidator. The current stabilization fee is added to the total liabilities of the liquidator
        let unpaid_stable_fee = self.internal_user_unpaid_stable(account.clone());

        let _allot_debt = allot_debt.checked_add(unpaid_stable_fee).expect(ERR_ADD);
        
        assert!(self.total_token > allot_token, "No liquidation");

        let mut reduce_pledge       = 0u128;//The system actually deducts the token
        let mut reduce_coin         = 0u128;//The system actually deducts coins
        let mut surplus_token       = 0u128;//The remaining token is sent to the liquidator
        let mut stake_allot_coin    = 0u128;//The pledge pool allocates debt
        let mut stake_allot_token   = 0u128;//Pledge pool allocates pledges
        let mut sp_allot_coin       = 0u128;//The stabilization pool allocates debt
        let mut sp_allot_token      = 0u128;//Stable pool allocates pledges
        let mut sys_reduce_token    = 0u128;//The system deducts tokens for calculation
        let mut liquidation_fee     = 0u128;//Clearing charge
        
        //5. Liquidation penalty line is 110.5%, and the liquidator is given priority of gas compensation of 0.5%
        // When the customer pledge rate is greater than or equal to 110.5%, the project party will charge 2% clearing commission
        // When the customer pledge rate is less than 110.5%, the project party will not charge the clearing commission (2%)

        let total_usdo_deposits = self._get_total_usdo_deposits();
        //Clearing gas compensation = personal pledge *0.5%
        let gas_compensation = _allot_debt//8
                            .checked_mul(self.gas_compensation_ratio).expect(ERR_MUL)//8
                            .checked_mul(STAKE_RATIO_BASE).expect(ERR_MUL)//16
                            .checked_div(self.token_price).expect(ERR_DIV);
        //Liquidation commission (The project party) shall calculate the liquidation commission if the current customer pledge rate is greater than or equal to 110.5%
        if ratio >= INIT_NO_LIQUIDATION_FEE_RATE {
            liquidation_fee = _allot_debt
                            .checked_mul(self.liquidation_fee_ratio).expect(ERR_MUL)
                            .checked_mul(STAKE_RATIO_BASE).expect(ERR_MUL)//16
                            .checked_div(self.token_price).expect(ERR_DIV);
        }

        let token = allot_token
                    .checked_sub(gas_compensation).expect(ERR_SUB)
                    .checked_sub(liquidation_fee).expect(ERR_SUB);
 
        if total_usdo_deposits == 0 {

            //The stable pool is empty
            stake_allot_coin  = _allot_debt;
            stake_allot_token = self.stable_stake_allot(_allot_debt, token , ratio, false , false);

             //Pledges allocated by the pledge pool
            surplus_token   = token.checked_sub(stake_allot_token).expect(ERR_SUB);
            //Deduct the pledge liquidation_gas + surplus_token; 
            reduce_pledge   = allot_token.checked_sub(stake_allot_token).expect(ERR_SUB);
            
            sys_reduce_token  = allot_token;//The system actually reduces the token
            
        } else if total_usdo_deposits > 0 && total_usdo_deposits < _allot_debt {

            //The stabilization pool is smaller than the current liabilities
            sp_allot_coin  = total_usdo_deposits;//The stable pool allocates coins
            sp_allot_token = self.stable_stake_allot(total_usdo_deposits, token, ratio, true , false);
            
            stake_allot_coin= _allot_debt.checked_sub(total_usdo_deposits).expect(ERR_SUB);
            stake_allot_token  = self.stable_stake_allot(stake_allot_coin, token - sp_allot_token, ratio , false , false);

            surplus_token = token
                            .checked_sub(sp_allot_token).expect(ERR_SUB)
                            .checked_sub(stake_allot_token).expect(ERR_SUB);

            reduce_pledge = allot_token.checked_sub(stake_allot_token).expect(ERR_SUB);
            reduce_coin   = total_usdo_deposits;

            sys_reduce_token   = allot_token;//The system actually reduces the token

        } else if total_usdo_deposits > 0 && total_usdo_deposits >= allot_debt {

            //The stability pool is larger than the current liabilities
            sp_allot_coin  = _allot_debt;//The stable pool allocates coins
            sp_allot_token = self.stable_stake_allot(_allot_debt, token, ratio , true , true);//The stable pool allocates tokens

            surplus_token = token.checked_sub(sp_allot_token).expect(ERR_SUB);
            reduce_pledge = allot_token;
            reduce_coin   = _allot_debt;
        }

        //Stable pool allocation
        self._offset(sp_allot_coin, sp_allot_token);
        //Debt pool allocation
        self.liquidation_debt_allot(stake_allot_coin, stake_allot_token, sys_reduce_token);
        
        self.total_token = self.total_token.checked_sub(reduce_pledge).expect(ERR_SUB);
        self.total_guarantee = self.total_guarantee
            .checked_sub(guarantee).expect(ERR_SUB);

        self.total_coin = self.total_coin
            .checked_add(guarantee.into()).expect(ERR_ADD)
            .checked_add(unpaid_stable_fee.into()).expect(ERR_ADD)
            .checked_sub(reduce_coin.into()).expect(ERR_SUB);

        //System unpaid total stabilization fee reduced
        self.total_unpaid_stable_fee = self.total_unpaid_stable_fee.checked_sub(unpaid_stable_fee.into()).expect(ERR_SUB);

        //Reset the user
        self.default_account(account.clone());
        //To delete a linked list
        self._vault_remove(account.clone().try_into().unwrap());
        
        //Casting gas compensation
        self.internal_mint(account_id.clone(), guarantee.into());

        //Dispose the remaining token of gas compensation liquidation fee as mining reward
        self.personal_liquidation_token(account_id.clone(), account.clone(), gas_compensation, surplus_token, liquidation_fee);

        //If the system pledge rate is too low, system functions are suspended
        if self.internal_sys_ratio() <= INIT_MIN_RATIO_LINE {
            self.internal_shutdown();
        }

        log!("_allot_debt {} allot_debt {} unpaid_stable_fee {}", _allot_debt, allot_debt, unpaid_stable_fee);

        log!("total_coin {} total_unpaid_stable_fee {} ", self.total_coin, self.total_unpaid_stable_fee);

        log!("Current total clearing TOKEN: {}, clearing GAS compensation: {:? }, liquidation fee: {:? }, remaining TOKEN: {:? }, stable pool allocation: (debt: {}, token: {}), debt pool allocation: (debt: {}, token: {}), remaining users settle token: {:? }, the system actually reduces TOKEN: {}, the system actually reduces COIN: {}", allot_token, gas_compensation, liquidation_fee, token, sp_allot_coin,sp_allot_token, stake_allot_coin,stake_allot_token, surplus_token, reduce_pledge, reduce_coin);
    }

    pub(crate) fn get_debt(&self, account: AccountId) -> (u128, u128, u128) {
        let account_coin = self.account_coin.get(&account).expect(ERR_NOT_REGISTER);
        let account_token = self.account_token.get(&account).expect(ERR_NOT_REGISTER);
        let guarantee = self.guarantee.get(&account).expect(ERR_NOT_REGISTER);
        let (allot_debt, allot_token) = self.get_account_allot(account.clone());

        let debt = account_coin
            .checked_add(guarantee).expect(ERR_ADD)
            .checked_add(allot_debt).expect(ERR_ADD);
        let token = account_token.checked_add(allot_token).expect(ERR_ADD);
        //Return (user debt, user pledge, user deposit)
        (debt, token, guarantee)
    }

    // TODO Ok->Gets parameters used to configure web pages
    
    pub(crate) fn cal_storage_near(&self) -> u128 {
        self.stake_storage_usage * env::storage_byte_cost()
    }

    #[payable]
    pub fn register_account(&mut self, account: AccountId) {
        assert!(
            env::attached_deposit() >= self.cal_storage_near(),
            "Stake storage usage not enough."
        );

        log!("GAS：{}", self.cal_storage_near());

        if !self.is_register(account.clone()) {
            for (key, value) in self.reward_coins.iter() {
                self.account_reward.insert(
                    &self.get_staker_reward_key(account.clone(), key.clone()),
                    &UserReward {
                        index: value.double_scale,
                        reward: 0,
                    },
                );

            } 

            self.account_coin.insert(&account.clone(), &0);
            self.account_token
                .insert(&account.clone(), &0);
            self.guarantee.insert(&account.clone(), &0);

            self.account_stable.insert(
                &account.clone(),
                &UserStable {
                    unpaid_stable_fee: 0,
                    index: 0,
                },
            );
            self.account_allot.insert(
                &account.clone(),
                &AccountAllot {
                    account_allot_debt: 0,
                    account_allot_token: 0,
                },
            );

            self.register_usdo(account.clone());

            self.min_reward
                .insert(&account.clone(), &0);
            self.dis_reward
                .insert(&account.clone(), &0);

            let current_p = self.p_system.clone();
            let scale_current = self.current_scale.clone();
            let epoch_current = self.current_epoch.clone();
            self.deposits.insert(&account.clone(), &0);
            self.deposit_snapshots.insert(
                &account.clone(),
                &Snapshots {
                    scale: scale_current,
                    epoch: epoch_current,
                    p: current_p,
                    g: self
                        .epoch_to_scale_to_g
                        .get(&epoch_current)
                        .expect(ERR_GET)
                        .get(&scale_current)
                        .expect(ERR_GET)
                        .g_system,
                    s: self
                        .epoch_to_scale_to_sum
                        .get(&epoch_current)
                        .expect(ERR_GET)
                        .get(&scale_current)
                        .expect(ERR_GET),
                },
            );
        } else {
            env::panic("The account is register yet.".as_ref());
        }
    }

    pub fn is_register(&self, account: AccountId) -> bool {
        self.account_token.contains_key(&account)
    }

    // TODO Ok
    pub fn ft_on_transfer(
        &mut self,
        sender_id: ValidAccountId,
        amount: U128,
        msg: String, /* token */
    ) -> PromiseOrValue<U128> {
        let args: FtOnTransferArgs = serde_json::from_str(&msg).expect(ERR_PARSE);
        let token_account_id = env::predecessor_account_id();
        let amount_return;
        match args {
            FtOnTransferArgs::DepositToken => {
                assert_eq!(token_account_id, ST_NEAR, "only surpport St_near!");
                assert!(self.is_register(sender_id.clone().into()), "{}", ERR_NOT_REGISTER);

                self.deposit_token(amount.0, sender_id);
                amount_return = 0;
            }

            FtOnTransferArgs::InjectReward => {
                //Determine if it is a reward currency
                self.find_reward_coin(token_account_id.clone());
                
                assert!(self.is_owner(sender_id.clone().into()), "only owner can do this!");

                self.inject_reward(amount, token_account_id.clone());
                amount_return = 0;
            }

            FtOnTransferArgs::InjectSpReward => {
                assert_eq!(token_account_id, ST_OIN, "only surpport OIN!");
                assert!(self.is_owner(sender_id.clone().into()), "only owner can do this!");

                self.inject_sp_reward(amount, sender_id);
                amount_return = 0;
            }
        }
        PromiseOrValue::Value(amount_return.into())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum FtOnTransferArgs {
    DepositToken,
    InjectSpReward,
    InjectReward,
}
