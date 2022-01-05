use near_sdk::json_types::{Base58PublicKey, U128};
use near_sdk::{env, near_bindgen, AccountId};

use std::convert::TryFrom;

use crate::*;

#[near_bindgen]
impl OinStake {
    // TODO[OK] Get the total usDO of the stable pool
    pub(crate) fn _get_total_usdo_deposits(&self) -> u128 {
        self.total_usdo_deposits
    }

    // TODO[OK] Get reward token information
    pub(crate) fn internal_get_reward_coin(&self, coin_key: AccountId) -> RewardCoin {
        self.reward_coins.get(&coin_key).expect(ERR_NO_COIN)
    }
    
    // TODO[OK] Get user reward -> User =>coin information
    pub(crate) fn internal_get_account_reward(
        &self,
        staker: AccountId,
        coin_key: AccountId,
    ) -> UserReward {
        self.account_reward
            .get(&self.get_staker_reward_key(staker, coin_key))
            .expect(ERR_NOT_REGISTER_REWARD)
    }
    pub fn get_account_reward(
        &self,
        staker: AccountId,
        reward_coin: AccountId,
    ) -> Option<UserReward> {
        self.account_reward
            .get(&self.get_staker_reward_key(staker, reward_coin))
    }

    pub fn get_staker_reward_key(&self, staker: AccountId, reward_coin: AccountId) -> String {
        format!("{}:{}", staker, reward_coin)
    }

    // TODO[OK] Calculation of reward
    pub(crate) fn internal_get_saved_reward(
        &self,
        staker: AccountId,
        coin_key: AccountId,
    ) -> u128 {
        let reward_coin_ins = self.internal_get_reward_coin(coin_key.clone());
        let stake_token_num = self.account_token.get(&staker.clone()).expect(ERR_NOT_REGISTER);

        if let Some(user_reward) = self
            .account_reward
            .get(&self.get_staker_reward_key(staker.clone(), coin_key.clone()))
        {
            user_reward
                .reward
                .checked_add(
                    U256::from(
                        reward_coin_ins
                            .index
                            .checked_sub(user_reward.index)
                            .expect(ERR_SUB),
                    )
                    .checked_mul(U256::from(stake_token_num))
                    .expect(ERR_MUL)
                    .checked_div(U256::from(reward_coin_ins.double_scale))
                    .expect(ERR_DIV)
                    .as_u128(),
                )
                .expect(ERR_ADD)
        } else {
            0
        }
    }
    pub(crate) fn internal_get_unsaved_reward(
        &self,
        staker: AccountId,
        coin_key: AccountId,
    ) -> u128 {
        let reward_coin_ins = self.internal_get_reward_coin(coin_key.clone());
        let stake_token_num = self.account_token.get(&staker.clone()).expect(ERR_NOT_REGISTER);
        let current_block_number = self.to_nano( env::block_timestamp());

        if self.total_token == 0 {
            0
        } else {
            U256::from(
                (current_block_number as u128)
                    .checked_sub(reward_coin_ins.block_number as u128)
                    .expect(ERR_SUB),
            )
            .checked_mul(U256::from(reward_coin_ins.reward_speed))
            .expect(ERR_MUL)
            .checked_mul(U256::from(stake_token_num))
            .expect(ERR_MUL)
            .checked_div(U256::from(self.total_token))
            .expect(ERR_DIV)
            .as_u128()
        }
    }
    pub(crate) fn internal_get_staker_reward(
        &self,
        staker: AccountId,
        coin_key: AccountId,
    ) -> u128 {
        self.internal_get_saved_reward(staker.clone(), coin_key.clone())
            .checked_add(self.internal_get_unsaved_reward(staker.clone(), coin_key.clone()))
            .expect(ERR_ADD)
    }

    /*Gets the currency currently pledged*/
    pub fn get_stake_token(&self) -> String {
        self.stake_token.to_string()
    }

    /*Gets the address of the Oin in which the stability fee is paid*/
    pub fn get_stable_fee_token(&self) -> String {
        self.stable_token.to_string()
    }

    //stablepool
    // Gets the current user's mining bonus total
    pub fn get_depositor_block_near_gain(&self, account: AccountId) -> U128 {
        //Here you get a mining reward
        let near_gain_block = self._get_near_block_gain_from_snapshots(&account);

        U128(near_gain_block)
    }

    /*Stable pool mining bonus speed*/
    pub fn get_reward_speed_sp(&self) -> U128 {
        U128(self.reward_speed_sp)
    }
    
    /*Stable pool mining bonus total*/
    pub fn get_reward_sp(&self) -> U128 {
        U128(self.reward_sp)
    }

    /*Gets the remaining amount of each USDO of the current stable pool user*/
    pub fn get_compounded_usdo_deposit(&self, account: AccountId) -> u128 {
        self._get_compounded_usdo_deposit(account)
    }

    /*Get the total usDO of the stable pool*/
    pub fn get_total_usdo_deposits(&self) -> U128 {
        U128(self._get_total_usdo_deposits())
    }

    // TODO[OK] Minimum amount pledged FRONT
    pub fn min_amount_token(&self) -> U128 {
        U128(self._min_amount_token())
    }

    // TODO[OK] Minimum amount produced FRONT
    pub fn min_amount_coin(&self) -> U128 {
        U128(self.min_mint_amount)
    }

    // TODO[OK] guarantee FRONT
    pub fn get_guarantee_limit(&self) -> U128 {
        U128(self.guarantee_limit)
    }

    // TODO[OK] FRONT
    pub fn get_available_token(&self, account: AccountId) -> U128 {
        U128(self.internal_avaliable_token(account))
    }
    // TODO[OK] FRONT
    pub fn get_available_coin(&self, account: AccountId) -> U128 {
        U128(self.internal_can_mint_amount(account))
    }
    // TODO[OK] FRONT
    pub fn get_user_unpaid_stable(&self, account: AccountId) -> U128 {
        U128(self.internal_user_unpaid_stable(account))
    }
    // TODO[OK] FRONT
    pub fn get_sys_ratio(&self) -> U128 {
        U128(self.internal_sys_ratio())
    }
    // TODO[OK] FRONT
    pub fn get_user_ratio(&self, account: AccountId) -> U128 {
        U128(self.internal_user_ratio(account))
    }
    // TODO[OK] FRONT
    pub fn get_stable_fee_ratio(&self) -> U128 {
        U128(self.stable_fee_rate)
    }

    // TODO[OK] FRONT Get the user's latest debt
    pub fn external_staker_debt_of(&self, staker: AccountId) -> (U128, U128, U128) {
        let (dept, token, guarantee) = self.get_debt(staker);
        (U128(token), U128(dept), U128(guarantee))
    }

    // TODO[OK] FRONT
    pub fn get_liquidation_line(&self) -> U128 {
        U128(self.liquidation_line)
    }

    // TODO[OK] FRONT
    pub fn get_coin_upper_limit(&self) -> U128 {
        U128(self.coin_upper_limit)
    }
 
    // TODO[OK] FRONT
    pub fn get_oracle_time(&self) -> U128 {
        U128(self.token_poke_time as u128)
    }
    // TODO[OK] FRONT
    pub fn get_block_number(&self) -> U128 {
        let current_block_number = self.to_nano( env::block_timestamp());
        U128(current_block_number as u128)
    }

    // TODO[OK] FRONT
    pub fn get_total_token(&self) -> U128 {
        U128(self.total_token)
    }

    // TODO[OK] FRONT
    pub fn get_total_coin(&self) -> U128 {
        U128(self.total_coin)
    }

    // TODO[OK] FRONT
    pub fn get_saved_reward(
        &self,
        staker: AccountId,
        reward_coin: AccountId,
    ) -> U128 {
        U128(self.internal_get_saved_reward(staker, reward_coin))
    }

    // TODO[OK] FRONT
    pub fn get_unsaved_reward(&self, staker: AccountId, reward_coin: AccountId) -> U128 {
        U128(self.internal_get_unsaved_reward(staker, reward_coin))
    }

    // TODO[OK] FRONT
    pub fn get_staker_reward(&self, staker: AccountId, reward_coin: AccountId) -> U128 {
        U128(self.internal_get_staker_reward(staker, reward_coin))
    }
    
    // TODO[OK] FRONT Get the total amount of system reward tokens
    pub fn get_reward_coin(&self, reward_coin: AccountId) -> U128 {
        let reward_coin_ins = self.internal_get_reward_coin(reward_coin);
        U128(reward_coin_ins.total_reward)
    }

    // TODO[OK] FRONT
    pub fn register_storage_used(&self) -> U128 {
        U128(self.cal_storage_near())
    }

    // TODO[OK] FRONT
    pub fn register_usdo_used(&self) -> U128 {
        U128(self.usdo_storage_usage())
    }

    // TODO[OK] Get the USDO balance method
    pub fn get_usdo_balance(&self, account_id: AccountId) -> U128 {
        U128(self.ft_balance(account_id))
    }

    pub fn is_owner(&self, account: AccountId) -> bool {
        self.owner_id == account
    }
    
    pub fn external_get_dept(&self, account_id: AccountId) -> (U128, U128, U128) {
        let (dept, token, guarantee) = self.get_debt(account_id);
        (U128(dept), U128(token), U128(guarantee))
    }

    /**multisign */
    pub fn is_confirmed(&self, request_id: RequestId, account_pk: Base58PublicKey) -> bool {
        let confirmations = self.confirmations.get(&request_id).unwrap();
        confirmations.contains(&account_pk.0)
    }

    pub fn get_request(&self, request_id: RequestId) -> MultiSigRequest {
        (self.requests.get(&request_id).expect("No such request")).request
    }

    pub fn get_num_requests_pk(&self, public_key: Base58PublicKey) -> u32 {
        self.num_requests_pk.get(&public_key.into()).unwrap_or(0)
    }

    pub fn list_request_ids(&self) -> Vec<RequestId> {
        self.requests.keys().collect()
    }

    pub fn get_confirmed_num(&self, request_id: RequestId) -> u32 {
        self.confirmations
            .get(&request_id)
            .expect(ERR_GET)
            .len() as u32
    }
 
    pub fn get_confirmations(&self, request_id: RequestId) -> Vec<Base58PublicKey> {
        self.confirmations
            .get(&request_id)
            .expect(ERR_GET)
            .into_iter()
            .map(|key| Base58PublicKey::try_from(key).expect("Failed to covert key to base58"))
            .collect()
    }

    // 0 vote 1 confirm 2 right 3 expired
    pub fn req_status(&self, request_id: RequestId) -> u32 {
        
        let request = self.requests.get(&request_id).unwrap();
        let time_status = (env::block_timestamp() - request.added_timestamp) >= REQUEST_EXPIRE_TIME;

        if request.is_executed {
            2
        } else {
            if self.is_num_enough(request_id) {
                1
            } else {
                if time_status {
                    3
                } else {
                    0
                }
            }
        }
    }

    pub fn is_mul_white(&self, account: AccountId) -> bool {
        self.mul_white_list.contains(&account)
    }

    // num of multsigner
    pub fn mul_white_num(&self) -> u64 {
        self.mul_white_list.len()
    }

    // Get the total stabilization fee
    pub fn get_total_stable_fee(&self) -> U128 {
        U128(self.total_paid_stable_fee)
    }

    // Get the total liquidation fee
    pub fn get_total_liquidation_fee(&self) -> U128 {
        U128(self.total_liquidation_fee)
    }
    
    
}
