use near_sdk::{env, near_bindgen};

use crate::*;
/*
   token_price
   token_poke_time
   oracle_whitelist
*/
#[near_bindgen]
impl OinStake {
    // TODO Push the price [Ok]
    pub fn poke(&mut self, token_price: U128) {
        self.assert_oracle();
        self.token_price = token_price.into();
     
        self.token_poke_time = env::block_timestamp();
        //Determine whether there is currently a pledge and if there is, determine whether the system pledge rate is lower than the minimum pledge rate is lower than all the functions of the suspension system
       if self.total_token > 0 {
           if self.internal_sys_ratio() <= INIT_MIN_RATIO_LINE {
                self.internal_shutdown();
           }
       }

        log!(
            "{} poke price {} successfully.",
            env::predecessor_account_id(),
            token_price.0
        );
    }

    // TODO Get price - external [Ok]
    pub fn peek(&self) -> U128 {
        U128(self.token_price)
    }

    // TODO Assert whether push... [Ok]
    pub(crate) fn assert_is_poked(&self) {
        assert!(self.token_price != 0 && env::block_timestamp().checked_sub(self.token_poke_time).expect(ERR_SUB) < POKE_INTERVAL_TIME, "Token price isn't poked.");
    }

    pub fn set_oracle(&mut self, account_id: ValidAccountId) {
        self.assert_owner();
        let oracle:AccountId = account_id.into();
        assert!(
            self.oracle != oracle.clone(),
            "The account is oracle yet."
        );

        self.oracle = oracle.clone();
        log!("{} add oracle {}", self.get_owner(), oracle);
    }

    pub(crate) fn assert_oracle(&self) {
        assert!(
            self.oracle == env::predecessor_account_id(),
            "Only oracle user can do it."
        );
    }

}
