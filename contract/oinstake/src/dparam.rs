use crate::*;
use near_sdk::json_types::{U128, U64};
use near_sdk::near_bindgen;

#[near_bindgen]
impl OinStake {

    // TODO [OK] Set a steady rate
    #[private]
    pub fn set_stable_fee_rate(&mut self, fee_rate: U128) {

        self.assert_not_current();
        assert!(fee_rate.0 <= MAX_STABLE_FEE_RATE, "Exceeding the maximum setting");
        self.stable_fee_rate = fee_rate.into();
        //Updated the index value of the stability fee system
        self.update_stable_system_index();
        log!("Set stable fee rate {}", fee_rate.0);

    }
    // TODO [OK]  Set up clearing line
    #[private]
    pub fn set_liquidation_line(&mut self, liquidation_line: U128) {
        self.assert_not_current();
        assert!(liquidation_line.0 >= MIN_LIQUIDATION_LINE,"Exceeding the maximum setting");
        self.liquidation_line = liquidation_line.into();
        log!("Set liquidation line {}", liquidation_line.0);
    }

    // TODO [OK] COIN Hard top
    #[private]
    pub fn set_coin_upper_limit(&mut self, coin_upper_limit: U128) {
        self.assert_not_current();
        assert!(coin_upper_limit.0 <= MAX_COIN_UPPER_LIMIT, "Exceeding the maximum setting");
        assert!(coin_upper_limit.0 >= self.total_coin, "The value cannot be less than the total coin of the current system");
        self.coin_upper_limit = coin_upper_limit.into();
        log!("Set coin upper limit {}", coin_upper_limit.0);
    }

    // todo [OK] Minimum yield setting
    #[private]
    pub fn set_min_mint_amount(&mut self, amount: U128) {
        self.assert_not_current();
        self.min_mint_amount = amount.0;
        log!("Set minimum pledge amount {}", amount.0);
    }

    //TODO [OK] Personal Deposit setting
    #[private]
    pub fn set_guarantee(&mut self, amount: U128) {
        self.assert_not_current();
        self.guarantee_limit = amount.0;
        log!("Set total guarantee amount {}", amount.0);
    }

    //TODO [OK] Debt allocation ratio setting
    #[private]
    pub fn set_allot_ratio(&mut self, amount: U128) {
        self.assert_not_current();
        assert!(amount.0 >= MIN_ALLOT_RATIO, "Exceeding the minimum setting");
        assert!(amount.0 <= MAX_ALLOT_RATIO, "Exceeding the maximum setting");
        self.allot_ratio = amount.0;
        log!("Set allot ratio {}", amount.0);
    }

    //TODO [OK] Gas compensation Settings
    #[private]
    pub fn set_gas_compensation_ratio(&mut self, amount: U128) {
        self.assert_not_current();
        assert!(amount.0 > 0, "Exceeding the minimum setting");
        assert!(amount.0 <= MAX_GAS_COMPENSATION_RATIO, "Exceeding the maximum setting");
        self.gas_compensation_ratio = amount.0;
        log!("Set gas_compensation_ratio {}", amount.0);
    }

    //TODO [OK] Clearing charge
    #[private]
    pub fn set_liquidation_fee_ratio(&mut self, amount: U128) {
        self.assert_not_current();
        assert!(amount.0 > 0, "Exceeding the minimum setting");
        assert!(amount.0 <= MAX_LIQUIDATION_FEE_RATIO, "Exceeding the maximum setting");
        self.liquidation_fee_ratio = amount.0;
        log!("Set liquidation fee ratio {}", amount.0);
    }

    //TODO [OK] Update bonus speed
    pub fn set_reward_speed(&mut self, reward_coin: AccountId, speed: U128) {
        self.assert_white();
        self.internal_set_reward_speed(reward_coin, speed);
    }

    //  update reward speed of stable pool
    pub fn set_sp_reward_speed(&mut self, speed: U128) {
        self.assert_white();

        self._update_epoch_to_scale_to_g();

        self.reward_speed_sp = speed.0;

        log!(
            "{} set sp_reward_speed  to {}",
            env::predecessor_account_id(),
            speed.0
        );
    }

    /**multisign */
    //Add a multi-sign-on administrator
    pub fn add_mul_white(&mut self, account: AccountId) {
        self.assert_owner();
        assert!(
            !self.is_mul_white(account.clone()),
            "The account was in whitelist yet."
        );

        self.mul_white_list.insert(&account);
        log!("{} add mul_white {}", self.get_owner(), account);
    }

    //Example Remove an administrator with multiple signatures
    pub fn remove_mul_white(&mut self, account: AccountId) {
        self.assert_owner();
        assert!(
            self.is_mul_white(account.clone()),
            "The account isn't in whitelist."
        );

        self.mul_white_list.remove(&account);
        log!("{} remove mul_white {}", self.get_owner(), account);
    }

    //You can only call to set the cooldown time for multiple check-in requests
    #[private]
    pub fn set_request_cooldown(&mut self, amount: U64) {
        self.assert_not_current();
        let value = amount.0;
        assert!(
            value >= DEFAULT_REQUEST_COOLDOWN && value <= MAX_REQUEST_COOLDOWN,
            "illegal num_condirm_ratio"
        );
        self.request_cooldown = amount.0;
    }
    //You can only call to set the pass ratio of multiple sign-in requests. By default, 60 or 60% of people can pass it
    #[private]
    pub fn set_num_confirm_ratio(&mut self, amount: U64) {
        self.assert_not_current();
        let value = amount.0;
        assert!(value >= 50 && value <= 100, "unleagal num_condirm_ratio");
        self.num_confirm_ratio = value;
    }

     //Determine non-current contract accounts
     fn assert_not_current(&mut self) {
        assert!(env::signer_account_id()!= env::current_account_id(), "not allow current account to use！");
    }
}
