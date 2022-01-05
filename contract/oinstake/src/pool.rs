use near_sdk::json_types::U128;
use near_sdk::{env, ext_contract, near_bindgen, AccountId};

use crate::*;

#[ext_contract(ext_reward)]
pub trait ExtReward {
    fn on_extract_reward(&mut self, reward_coin: AccountId, amount: U128);
}

#[near_bindgen]
impl OinStake {
    // TODO [OK] Injection of reward 
    pub(crate) fn inject_reward(&mut self, amount: U128, reward_coin: AccountId) {
        let  (key, mut reward_coin_ins) = self.find_reward_coin(reward_coin.clone());

        reward_coin_ins.total_reward = reward_coin_ins
            .total_reward
            .checked_add(amount.into())
            .expect(ERR_ADD);
        self.reward_coins.insert(&key, &reward_coin_ins);
        log!("inject{} reward token{}", amount.0, reward_coin);
    }

    // TODO [OK] Extract the reward
    #[payable]
    pub fn extract_reward(&mut self, account: AccountId, amount: U128, reward_coin: AccountId) {
        self.assert_owner();

        let coin_key = reward_coin;
        let mut reward_coin_ins = self.internal_get_reward_coin(coin_key.clone());

        assert!(
            amount.0 <= reward_coin_ins.total_reward,
            "Withdraw overflow"
        );

        reward_coin_ins.total_reward = reward_coin_ins
            .total_reward
            .checked_sub(amount.into())
            .expect(ERR_ADD);
        self.reward_coins.insert(&coin_key, &reward_coin_ins);

        ext_fungible_token::ft_transfer(
            account,
            amount,
            None,
            &reward_coin_ins.token,
            ONE_YOCTO_NEAR,
            GAS_FOR_FT_TRANSFER_CALL,
        )
        .then(ext_reward::on_extract_reward(
            coin_key,
            amount,
            &env::current_account_id(),
            0,
            GAS_FOR_FT_TRANSFER_CALL,
        ));
    }

    /*
       TODO [OK] Add bonus tokens initially key=>value 
        The initial key is equal to value
        The value can be modified
    */
    pub(crate) fn init_reward_coin(&mut self) {
        self.internal_add_reward_coin(RewardCoin {
            token: ST_NEAR.to_string(),
            total_reward: 0,
            // stakepool init to 0
            reward_speed: 0,
            index: INIT_INDEX,
            block_number: self.to_nano(env::block_timestamp()),
            double_scale: INIT_INDEX,
        })
    }

    pub(crate) fn internal_add_reward_coin(&mut self, coin: RewardCoin) {
        assert!(
            self.reward_coins.len() < REWARD_UPPER_BOUND,
            "The currency slot has been used up, please modify other currency information as appropriate",
        );

        match self.reward_coins.get(&coin.token) {
            Some(_) => {
                env::panic(b"The current currency has been added, please add a new currency.");
            }
            None => {}
        }
        self.reward_coins.insert(&coin.token, &coin);

        log!(
            "{} add the RewardCoin=> {:?}",
            env::predecessor_account_id(),
            coin
        )
    }

    // TODO [OK] Add bonus tokens
    pub fn add_reward_coin(&mut self, token: AccountId, reward_speed: U128, double_scale: U128) {
        self.assert_white();
        self.internal_add_reward_coin(RewardCoin {
            token,
            total_reward: 0,
            reward_speed: reward_speed.into(),
            index: double_scale.into(),
            block_number: self.to_nano(env::block_timestamp()),
            double_scale: double_scale.into(),
        })
    }

    // TODO [OK] Update bonus speed
    pub(crate) fn internal_set_reward_speed(&mut self, coin_key: AccountId, speed: U128) {
        self.update_system_single_index(coin_key.clone());
        let mut reward_coin_ins = self.reward_coins.get(&coin_key).expect(ERR_NO_COIN);
        reward_coin_ins.reward_speed = speed.into();
        self.reward_coins.insert(&coin_key, &reward_coin_ins);

        log!(
            "{} set reward coin {} speed to {}",
            env::predecessor_account_id(),
            coin_key,
            speed.0
        );
    }

    #[payable]
    //Old users re-register for other unregistered currencies
    pub fn register_other_token(&mut self, staker: AccountId) {
        self.assert_white();
        for (key, value) in self.reward_coins.iter() {
            let account_reward_key = self.get_staker_reward_key(staker.clone(), key.clone());
            if !self.account_reward.contains_key(&account_reward_key) {
                self.account_reward.insert(
                    &self.get_staker_reward_key(staker.clone(), key.clone()),
                    &UserReward {
                        index: value.double_scale,
                        reward: 0,
                    },
                );
            }
        }
        log!("Registered successfully");   
    }

    //Update the token
    //coin_key Currency subscript
    //reward_coin Reward currency token
    pub fn update_reward_token(&mut self, coin_key: AccountId, reward_coin: AccountId, double_scale: U128) {
        self.assert_white();
        let mut reward_coin_ins = self.reward_coins.get(&coin_key).expect(ERR_NO_COIN);
        log!("{} {}", reward_coin_ins.index , ONE_TOKEN);
        //Check whether the current speed is greater than 0
        assert!(reward_coin_ins.index == ONE_TOKEN && reward_coin_ins.reward_speed == 0, "The current currency is set and cannot be changed"); 

        reward_coin_ins.token = reward_coin.into();
        reward_coin_ins.double_scale = double_scale.into();
        self.reward_coins.insert(&coin_key, &reward_coin_ins);
        self.update_system_single_index(coin_key.clone());
    }

    #[private]
    pub fn on_extract_reward(&mut self, coin_key: AccountId, amount: U128) {
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(_) => {}
            PromiseResult::Failed => {
                let mut reward_coin_ins = self.internal_get_reward_coin(coin_key.clone());
                reward_coin_ins.total_reward = reward_coin_ins
                    .total_reward
                    .checked_add(amount.into())
                    .expect(ERR_ADD);
                self.reward_coins.insert(&coin_key, &reward_coin_ins);
            }
        };
    }

    //Determine if a reward currency exists
    pub(crate) fn find_reward_coin (&self,  reward_coin: AccountId) ->(AccountId, RewardCoin) {
        for ( key , value) in self.reward_coins.iter() {
            if value.token == reward_coin {
                return (key, value);
            }
        }
        env::panic(ERR_NO_COIN.as_ref());
    }

}
