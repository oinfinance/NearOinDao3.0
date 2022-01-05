use near_sdk::json_types::U128;
use near_sdk::{env, near_bindgen, AccountId};

use crate::*;

#[ext_contract(ext_user_reward)]
pub trait ExtUserReward {
    fn on_claim_reward(&mut self, holder: AccountId, coin_key: AccountId, amount: U128);
    fn on_claim_liquidation_fee_callback(&mut self, amount: U128);
    fn storage_deposit(&mut self, account_id: ValidAccountId, registration_only: Option<bool>);
}

#[near_bindgen]
impl OinStake {

    #[payable]
    //Settlement fee collection
    pub fn claim_liquidation_fee(&mut self) {
        self.assert_owner();
        
        //owner Collect all clearing charges
        let owner_claim_liquidation_fee = self.total_liquidation_fee;

        assert!(owner_claim_liquidation_fee > 0 , "amount Not enough！");

        self.total_liquidation_fee = 0;

        log!("{} claim {} amount {}", self.owner_id.clone(), ST_NEAR, owner_claim_liquidation_fee);

        ext_fungible_token::ft_transfer(
            self.owner_id.clone(),
            U128(owner_claim_liquidation_fee),
            None,
            &ST_NEAR,
            ONE_YOCTO_NEAR,
            GAS_FOR_FT_TRANSFER_CALL,
        )
        .then(ext_user_reward::on_claim_liquidation_fee_callback(
            U128(owner_claim_liquidation_fee),
            &env::current_account_id(),
            0,
            GAS_FOR_FT_TRANSFER_CALL,
        ));
    }

    //Stable pool bonus currency registration
    #[payable]
    pub fn reward_storage_deposit(&mut self,  reward_coin: AccountId, yocto_near: U128) {
        let staker = env::predecessor_account_id();
        ext_user_reward::storage_deposit(
            staker.clone().try_into().unwrap(),
            None,
            &reward_coin,
            yocto_near.into(),
            GAS_FOR_FT_TRANSFER_CALL,
        );
    }

    //Pledge pool reward currency registration
    #[payable]
    pub fn reward_stake_storage_deposit(&mut self,  reward_coin: AccountId, yocto_near: U128) {
        let staker = env::predecessor_account_id();
        let coin_key = reward_coin;
        let reward_coin_ins = self.internal_get_reward_coin(coin_key.clone());

        ext_user_reward::storage_deposit(
            staker.clone().try_into().unwrap(),
            None,
            &reward_coin_ins.token,
            yocto_near.into(),
            GAS_FOR_FT_TRANSFER_CALL,
        );
    }

    // TODO [OK]
    #[payable]
    pub fn claim_reward(&mut self, reward_coin: AccountId) {
        assert!(self.is_claim_reward_paused(), "{}", SYSTEM_PAUSE);
        let staker = env::predecessor_account_id();
        let coin_key = reward_coin;

        self.update_personal_token(staker.clone());
        let mut reward_coin_ins = self.internal_get_reward_coin(coin_key.clone());
        let mut user_reward_coin =
            self.internal_get_account_reward(staker.clone(), coin_key.clone());

        let value:u128;
        
        if reward_coin_ins.total_reward >= user_reward_coin.reward {
            value = user_reward_coin.reward;
        }else{
            value = reward_coin_ins.total_reward;
        }
        assert!(value > 0, "{}", ERR_REWARD_ZERO);

        reward_coin_ins.total_reward = reward_coin_ins
            .total_reward
            .checked_sub(value)
            .expect(ERR_SUB);

        user_reward_coin.index = reward_coin_ins.index;
        user_reward_coin.reward = user_reward_coin.reward.checked_sub(value).expect(ERR_SUB);

        self.account_reward.insert(
            &self.get_staker_reward_key(staker.clone(), coin_key.clone()),
            &user_reward_coin,
        );
        self.reward_coins.insert(&coin_key, &reward_coin_ins);

        log!("{} claim {} amount {}", staker.clone(), reward_coin_ins.token, value);

        ext_fungible_token::ft_transfer(
            staker.clone(),
            U128(value),
            None,
            &reward_coin_ins.token,
            ONE_YOCTO_NEAR,
            GAS_FOR_FT_TRANSFER_CALL,
        ).then(ext_user_reward::on_claim_reward(
            staker.clone(),
            coin_key,
            U128(value),
            &env::current_account_id(),
            0,
            GAS_FOR_FT_TRANSFER_CALL,
        ));
    }

    // TODO [OK]
    pub(crate) fn update_personal_token(&mut self, account: AccountId) {
        self.update_system_index();
        for (coin_key, reward_coin) in self.reward_coins.iter() {
            let account_reward_key = self.get_staker_reward_key(account.clone(), coin_key.clone());
            //Determines whether the user reward token is initialized
            if !self.account_reward.contains_key(&account_reward_key) {
                self.account_reward.insert(
                    &account_reward_key,
                    &UserReward {
                        index: reward_coin.index,
                        reward: 0,
                    },
                );
            } else {
                let account_reward = self.account_reward.get(&account_reward_key).expect(ERR_NOT_REGISTER);
                if reward_coin.index > account_reward.index {
                    self.account_reward.insert(
                        &account_reward_key,
                        &UserReward {
                            index: reward_coin.index,
                            reward: self.internal_get_saved_reward(account.clone(), coin_key.clone()),
                        },
                    );

                    log!(
                        "Update {} reward info => {:?}",
                        coin_key,
                        self.account_reward.get(&account_reward_key)
                    )
                }
            }
            
        }
    }

    // TODO [OK] Update system index for each reward currency
    pub(crate) fn update_system_index(&mut self) {
        let current_block_number = self.to_nano(env::block_timestamp());
        log!("The current time {}", current_block_number);
        let mut map = HashMap::new();
        for (coin_key, reward_coin) in self.reward_coins.iter() {
            let mut reward_coin = reward_coin;
            let delta_block = current_block_number
                .checked_sub(reward_coin.block_number)
                .expect(ERR_SUB);

            // TODO [OK] Time difference calculation rewards
            if delta_block > 0  && reward_coin.reward_speed > 0 {
                let total_increased_reward = reward_coin
                    .reward_speed
                    .checked_mul(delta_block as u128)
                    .expect(ERR_MUL);/*24*/

                let mut reward_gain_per_token = 0;
                if self.total_token > 0 {
                    //Calculate the number of rewards allocated per token
                    reward_gain_per_token = U256::from(total_increased_reward)
                        .checked_mul(U256::from(reward_coin.double_scale))
                        .expect(ERR_MUL)
                        .checked_div(U256::from(self.total_token))
                        .expect(ERR_DIV)
                        .as_u128();
                }
                reward_coin.index = reward_coin
                    .index
                    .checked_add(reward_gain_per_token)
                    .expect(ERR_ADD);
                reward_coin.block_number = current_block_number;
                map.insert(coin_key, reward_coin);
            }
        }

        for (coin_key, reward_coin) in map.iter() {
            self.reward_coins.insert(coin_key, reward_coin);
            log!(
                "System Update {} reward info => {:?}",
                &coin_key.clone(),
                &reward_coin.clone()
            )
        }
    }


    //Update system index of single reward currency
    pub(crate) fn update_system_single_index(&mut self, coin_key: AccountId) {
        let current_block_number = self.to_nano(env::block_timestamp());
        log!("The current time {}", current_block_number);
        let mut reward_coin = self.internal_get_reward_coin(coin_key.clone());
        let delta_block = current_block_number
            .checked_sub(reward_coin.block_number)
            .expect(ERR_SUB);
        // TODO [OK] Time difference calculation rewards
        let total_increased_reward = reward_coin
            .reward_speed
            .checked_mul(delta_block as u128)
            .expect(ERR_MUL);/*24*/

        let mut reward_gain_per_token = 0;
        if self.total_token > 0 {
            //Calculate the number of rewards allocated per token
            reward_gain_per_token = U256::from(total_increased_reward)
                .checked_mul(U256::from(reward_coin.double_scale))
                .expect(ERR_MUL)
                .checked_div(U256::from(self.total_token))
                .expect(ERR_DIV)
                .as_u128();
        }
        reward_coin.index = reward_coin
            .index
            .checked_add(reward_gain_per_token)
            .expect(ERR_ADD);
        reward_coin.block_number = current_block_number;

        self.reward_coins.insert(&coin_key, &reward_coin);
    }




    //TODO[OK] updates the liquidator's gas compensation, and the liquidator puts the remaining token into the reward for users to claim as a reward
    // liquidator send_id
    // Liquidated account_id
    // gas liquidation_gas compensation
    // Remaining token surplus_token
    pub(crate) fn personal_liquidation_token(&mut self, send_id: AccountId, account_id: AccountId, liquidation_gas: Balance, surplus_token: Balance, liquidation_fee: Balance) {
        
        //self.owner_id
        let coin_key = ST_NEAR.to_string();
        //System reward coefficient query
        let mut sys_reward_coin = self.internal_get_reward_coin(coin_key.clone());
        
        let account_reward_key_o = self.get_staker_reward_key(send_id.clone(), coin_key.clone());
        let user_reward_coin_o = self.internal_get_account_reward(send_id.clone(), coin_key.clone());
        
        self.account_reward.insert(
            &account_reward_key_o,
            &UserReward {
                index:  user_reward_coin_o.index,
                reward: user_reward_coin_o.reward.checked_add(liquidation_gas).expect(ERR_ADD),
            },
        );
        
        if surplus_token > 0 {
            let account_reward_key_t = self.get_staker_reward_key(account_id.clone(), coin_key.clone());
            let user_reward_coin_t = self.internal_get_account_reward(account_id.clone(), coin_key.clone());

            //The liquidator put the remaining token into the reward
            self.account_reward.insert(
                &account_reward_key_t,
                &UserReward {
                    index:  user_reward_coin_t.index,
                    reward: user_reward_coin_t.reward.checked_add(surplus_token).expect(ERR_ADD),
                },
            );
        }

        if liquidation_fee > 0 {
            //Increase the settlement fee charged by the project side
            self.total_liquidation_fee = self.total_liquidation_fee
            .checked_add(liquidation_fee).expect(ERR_ADD);
        }
       
        //Change the current currency system total rewards
        sys_reward_coin.total_reward = sys_reward_coin
            .total_reward
            .checked_add(liquidation_gas).expect(ERR_ADD)
            .checked_add(surplus_token).expect(ERR_ADD);
        
        self.reward_coins.insert(&coin_key, &sys_reward_coin);
    }
    
}
