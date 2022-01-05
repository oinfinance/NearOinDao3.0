use near_sdk::{env, near_bindgen, AccountId};

use crate::*;

#[near_bindgen]
impl OinStake {

    /*
        TODO [OK] The assigned debt is not included in the stabilization fee calculation. After the stabilization fee calculation, the assigned debt is added to Account_coin
        If there is no debt (coin=0), the user stability fee will not be calculated, but the personal index will be updated to the system index
        If there is a current debt (coin>0) calculate the current user stability fee system index- personal index
    */

    pub(crate) fn update_stable_fee(&mut self, account: AccountId) {

        self.update_stable_system_index();

        if let Some(mut user_stable) = self.account_stable.get(&account) {

            let coin = self.account_coin.get(&account).expect(ERR_NOT_REGISTER);

            if coin > 0 {
                let fee = (self.stable_index
                        .checked_sub(user_stable.index).expect(ERR_SUB))//16
                        .checked_mul(coin).expect(ERR_MUL)//8
                        .checked_div(INIT_STABLE_INDEX).expect(ERR_DIV);//16
                       
                user_stable.unpaid_stable_fee = user_stable.unpaid_stable_fee
                        .checked_add(fee).expect(ERR_ADD); 
            }
            
            user_stable.index = self.stable_index;
            self.account_stable.insert(&account, &user_stable);
            log!("Current stabilization fee：{:?}",self.account_stable.get(&account));
        } else {
            env::panic(b"Not register");
        }
    }

    // TODO [OK] Updated the index value of the system stability charge
    pub fn update_stable_system_index(&mut self) {
     
        let current_block_number = self.to_nano(env::block_timestamp());

        let delta_block = current_block_number
            .checked_sub(self.stable_block_number)
            .expect(ERR_SUB);

        if delta_block > 0 && self.total_coin > 0 {
            // TODO [OK] Update index value currently calculates the stabilization fee charged at time difference for each debt
            self.stable_index = self.stable_index
                .checked_add(
                    self.stable_fee_rate
                        .checked_mul(delta_block  as u128).expect(ERR_MUL)
                        .checked_div(BLOCK_PER_YEAR).expect(ERR_DIV),
                )
                .expect(ERR_ADD);

            //TODO [OK] Record the total stabilization fees currently charged by the system
            self.total_unpaid_stable_fee = self.total_unpaid_stable_fee
            .checked_add(
                self.stable_fee_rate //16
                .checked_mul(delta_block as u128).expect(ERR_MUL)
                .checked_mul(self.total_coin).expect(ERR_MUL)
                .checked_div(BLOCK_PER_YEAR).expect(ERR_DIV)
                .checked_div(INIT_STABLE_INDEX).expect(ERR_DIV)
            )
            .expect(ERR_ADD);
        }

        log!("stable fee caculate: current sys time {},total_coin {} system index {}, total_unpaid_stable_fee {}",
            current_block_number,
            self.total_coin,
            self.stable_index,
            self.total_unpaid_stable_fee,
            );

        self.stable_block_number = current_block_number;
    }

}
