use near_sdk::json_types::U128;
use near_sdk::{env, log, near_bindgen, AccountId, Balance};

use std::convert::TryInto;

use crate::*;

#[near_bindgen]
impl OinStake {
    #[private]
    pub fn after_withdraw_token(
        &mut self,
        account_id: AccountId,
        amount: Balance,
        guarantee: Balance,
    ) {
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(_) => {
                log!(
                    "{:?} withdraw token amount {:?} Transfer success",
                    account_id.clone(),
                    amount
                );
            }
            PromiseResult::Failed => {
                let account_token = self
                    .account_token
                    .get(&account_id.clone())
                    .expect(ERR_NOT_REGISTER);

                self.total_token = self.total_token.checked_add(amount).expect(ERR_ADD);
                self.account_token.insert(
                    &account_id.clone(),
                    &(account_token.checked_add(amount).expect(ERR_ADD)),
                );

                if let Some(0) = self.guarantee.get(&account_id.clone()) {
                    self._vault_init(account_id.clone().try_into().unwrap());
                    self.total_guarantee = self
                        .total_guarantee
                        .checked_add(guarantee.clone())
                        .expect(ERR_ADD);
                    self.guarantee.insert(&account_id.clone(), &guarantee);
                }
                log!(
                    "{:?} withdraw token amount {:?}, Transfer failed",
                    account_id.clone(),
                    amount
                );
            }
        }
    }

    #[private]
    pub fn on_withdraw_usdo(
        &mut self,
        _receiver_id: AccountId,
        _amount: U128,
        _cha_g_bn: u64,
        _cha_g_system: InnerU256,
        _target_scale: u64,
        _target_epoch: U128,
        _before_deposits: U128,
        _before_snap: Snapshots,
    ) {
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(_) => {
                log!("send {:?} usdo to {:?}", _amount, _receiver_id);
            }
            PromiseResult::Failed => {
                let mut before_scale_g = self
                    .epoch_to_scale_to_g
                    .get(&_target_epoch.into())
                    .expect(ERR_GET);
                let mut before_system_g = before_scale_g.get(&_target_scale).expect(ERR_GET);

                before_system_g.g_block_num = before_system_g
                    .g_block_num
                    .checked_sub(_cha_g_bn)
                    .expect(ERR_SUB);
                before_system_g.g_system = (U256(before_system_g.g_system)
                    .checked_sub(U256(_cha_g_system))
                    .expect(ERR_SUB))
                .0;

                before_scale_g.insert(&_target_scale, &before_system_g);
                self.epoch_to_scale_to_g
                    .insert(&self.current_epoch, &before_scale_g);

                let new_total_deposits = self
                    .total_usdo_deposits
                    .checked_add(_amount.into())
                    .expect(ERR_ADD);
                self.total_usdo_deposits = new_total_deposits;

                self.deposit_snapshots.insert(&_receiver_id, &_before_snap);
                self.deposits
                    .insert(&_receiver_id, &_before_deposits.into());

                log!(
                    "System value before the failure：self.total_usdo_deposits{:?}g_block_num{:?}g_system{:?}",
                    self.total_usdo_deposits,
                    before_system_g.g_block_num,
                    U256(before_system_g.g_system.into())
                );

                log!("fail to transfer {:?} usdo to {:?}", _amount, _receiver_id);
            }
        };
    }

    #[private]
    pub fn on_claim_dis_reward(&mut self, _claimer: AccountId, _amount: U128) {
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(_) => {
                log!(
                    " transfer succeed,{:?} claim {:?} dis reward",
                    &_claimer,
                    _amount
                );
            }
            PromiseResult::Failed => {
                let depositor_near_gain = _amount.into();

                let unclaimed_dis_reward = self
                    .dis_reward
                    .get(&_claimer)
                    .expect(ERR_GET)
                    .checked_add(depositor_near_gain)
                    .expect(ERR_ADD);

                if depositor_near_gain > 0 {
                    self.total_token_sp = self
                        .total_token_sp
                        .checked_add(depositor_near_gain)
                        .expect(ERR_ADD);

                    self.dis_reward.insert(&_claimer, &unclaimed_dis_reward);
                }

                log!(
                    "{:?} dis reward {:?} transfer error",
                    _claimer,
                    depositor_near_gain
                );
            }
        }
    }

    #[private]
    pub fn on_claim_min_reward(
        &mut self,
        _claimer: AccountId,
        _token_to_claim: U128,
        _depositor_near_gain: U128,
    ) {
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(_) => {
                log!("transfer succeed , begin to change param");

                log!("{:?} claim {:?} min reward", _claimer, _token_to_claim);
            }
            PromiseResult::Failed => {
                let before_unclaimed_min = self.min_reward.get(&_claimer).expect(ERR_GET);
                let before_unclaimed_dis = self.dis_reward.get(&_claimer).expect(ERR_GET);

                self.min_reward.insert(
                    &_claimer,
                    &(before_unclaimed_min
                        .checked_add(_token_to_claim.into())
                        .expect(ERR_ADD)),
                );
                self.reward_sp = self
                    .reward_sp
                    .checked_add(_token_to_claim.into())
                    .expect(ERR_ADD);

                self.dis_reward.insert(
                    &_claimer,
                    &(before_unclaimed_dis
                        .checked_add(_depositor_near_gain.into())
                        .expect(ERR_ADD)),
                );

                self.total_token_sp = self
                    .total_token_sp
                    .checked_add(_depositor_near_gain.into())
                    .expect(ERR_ADD);

                log!(
                    "{:?} min reward {:?} transfer error",
                    _claimer,
                    _token_to_claim
                );
            }
        }
    }

    #[private]
    pub fn on_withdraw_sp_reward(&mut self, _receiver_id: AccountId, _amount: U128) {
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(_) => {
                log!("send {:?} stnear to {:?}", _amount, _receiver_id);
            }
            PromiseResult::Failed => {
                let amount = u128::from(_amount);

                self.reward_sp = self.reward_sp.checked_add(amount).expect(ERR_ADD);

                log!(
                    "fail to transfer {:?} stnear to {:?} current reward_sp {:?}",
                    u128::from(_amount),
                    _receiver_id,
                    self.reward_sp
                );
            }
        };
    }

    #[private]
    pub fn on_function_call(&mut self, request_id: RequestId) -> PromiseOrValue<bool> {
        match env::promise_result(0) {
            PromiseResult::NotReady => {
                return PromiseOrValue::Value(false);
            }
            PromiseResult::Successful(_) => {
                log!("request {:?} succeed exed", request_id);
                return PromiseOrValue::Value(true);
            }
            PromiseResult::Failed => {
                let mut request = self.requests.get(&request_id).expect(ERR_GET);
                request.is_executed = false;
                self.requests.insert(&request_id, &request);
                log!("failed to execute {:?} request ", request_id);
                return PromiseOrValue::Value(false);
            }
        };
    }

    #[private]
    pub fn on_mint_transfer(&mut self, holder: AccountId, amount: U128) {
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(_) => {
                log!("Success to mint {:?} Account: {:?}", amount, holder);
            }
            PromiseResult::Failed => {
                let coin = self.account_coin.get(&holder).expect(ERR_NOT_REGISTER);
                let user_coin_total = coin.checked_sub(amount.into()).expect(ERR_SUB);

                self.total_coin = self.total_coin.checked_sub(amount.into()).expect(ERR_SUB);
                self.account_coin.insert(&holder, &user_coin_total);
                self.internal_burn(env::current_account_id(), amount.into());
            }
        }
    }

    #[private]
    pub fn on_claim_reward(&mut self, holder: AccountId, coin_key: AccountId, amount: U128) {
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(_) => {
                log!("Transfer Success！");
            }
            PromiseResult::Failed => {
                let mut reward_coin_ins = self.internal_get_reward_coin(coin_key.clone());
                let mut user_reward_coin =
                    self.internal_get_account_reward(holder.clone(), coin_key.clone());
                reward_coin_ins.total_reward = reward_coin_ins
                    .total_reward
                    .checked_add(amount.into())
                    .expect(ERR_ADD);

                user_reward_coin.reward = user_reward_coin
                    .reward
                    .checked_add(amount.into())
                    .expect(ERR_ADD);

                self.account_reward.insert(&coin_key, &user_reward_coin);
                self.reward_coins.insert(&coin_key, &reward_coin_ins);
                log!("Transfer error, data callback.");
            }
        }
    }

    #[private]
    pub fn on_claim_liquidation_fee_callback(&mut self, amount: U128) {
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(_) => {
                log!("Transfer Success！");
            }
            PromiseResult::Failed => {
                self.total_liquidation_fee = self
                    .total_liquidation_fee
                    .checked_add(amount.into())
                    .expect(ERR_ADD);
                log!("Transfer error, data callback.");
            }
        }
    }
}
