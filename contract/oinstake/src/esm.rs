use crate::*;
use near_sdk::{env, near_bindgen};

/*
   stake_live
   redeem_live
*/

#[near_bindgen]
impl OinStake {
    // TODO [OK] 
    pub(crate) fn open_deposit(&mut self) {
        self.is_deposit_live = 1;
    }

    pub(crate) fn pause_deposit(&mut self) {
        self.is_deposit_live = 0;
    }
    
    pub(crate) fn open_withdraw(&mut self) {
        self.is_withdraw_live = 1;
    }

    pub(crate) fn pause_withdraw(&mut self) {
        self.is_withdraw_live = 0;
    }

    pub(crate) fn open_mint_coin(&mut self) {
        self.is_mint_coin_live = 1;
    }

    pub(crate) fn pause_mint_coin(&mut self) {
        self.is_mint_coin_live = 0;
    }

    pub(crate) fn open_burn_coin(&mut self) {
        self.is_burn_coin_live = 1;
    }

    pub(crate) fn pause_burn_coin(&mut self) {
        self.is_burn_coin_live = 0;
    }

    // TODO [OK]
    pub(crate) fn open_claim_reward(&mut self) {
        self.is_claim_reward_live = 1;
    }

    // TODO [OK]
    pub(crate) fn pause_claim_reward(&mut self) {
        self.is_claim_reward_live = 0;
    }

    // TODO [OK]
    pub(crate) fn open_liquidate(&mut self) {
        self.is_liquidate_live = 1;
    }

    // TODO [OK]
    pub(crate) fn pause_liquidate(&mut self) {
        self.is_liquidate_live = 0;
    }

    // TODO [OK]
    pub(crate) fn open_provide_to_sp(&mut self) {
        self.is_provide_to_sp_live = 1;
    }

    // TODO [OK]
    pub(crate) fn pause_provide_to_sp(&mut self) {
        self.is_provide_to_sp_live = 0;
    }

    // TODO [OK]
    pub(crate) fn open_withdraw_from_sp(&mut self) {
        self.is_withdraw_from_sp_live = 1;
    }

    // TODO [OK]
    pub(crate) fn pause_withdraw_from_sp(&mut self) {
        self.is_withdraw_from_sp_live = 0;
    }

    // TODO [OK]
    pub(crate) fn open_claim_token(&mut self) {
        self.is_claim_token_live = 1;
    }

    // TODO [OK]
    pub(crate) fn pause_claim_token(&mut self) {
        self.is_claim_token_live = 0;
    }


    // TODO [OK]
    pub fn is_deposit_paused(&self) -> bool {
        self.is_deposit_live == 1
    }

    // TODO [OK]
    pub fn is_withdraw_paused(&self) -> bool {
        self.is_withdraw_live == 1
    }

    // TODO [OK]
    pub fn is_mint_coin_paused(&self) -> bool {
        self.is_mint_coin_live == 1
    }

    // TODO [OK]
    pub fn is_burn_coin_paused(&self) -> bool {
        self.is_burn_coin_live == 1
    }

    // TODO [OK]
    pub fn is_claim_reward_paused(&self) -> bool {
        self.is_claim_reward_live == 1
    }

    // TODO [OK]
    pub fn is_liquidate_paused(&self) -> bool {
        self.is_liquidate_live == 1
    }

    // TODO [OK]
    pub fn is_provide_to_sp_paused(&self) -> bool {
        self.is_provide_to_sp_live == 1
    }

    // TODO [OK]
    pub fn is_withdraw_from_sp_paused(&self) -> bool {
        self.is_withdraw_from_sp_live == 1
    }

    // TODO [OK]
    pub fn is_claim_token_paused(&self) -> bool {
        self.is_claim_token_live == 1
    }

    //One button to enable pause
    #[private]
    pub fn internal_open(&mut self) {
        //Suspend the pledge to get the generation of redemption
        self.open_deposit();
        self.open_withdraw();
        self.open_mint_coin();
        self.open_burn_coin();
        self.open_claim_reward();
        self.open_liquidate();
        self.open_provide_to_sp();
        self.open_withdraw_from_sp();
        self.open_claim_token();
        log!(
            "{} open sys in {}",
            env::predecessor_account_id(),
            env::block_timestamp(),
        );
    }

    // TODO [OK]
    pub(crate) fn internal_shutdown(&mut self) {
        //Suspend the pledge to get the generation of redemption
        self.pause_deposit();
        self.pause_withdraw();
        self.pause_mint_coin();
        self.pause_burn_coin();
        self.pause_claim_reward();
        self.pause_liquidate();
        self.pause_provide_to_sp();
        self.pause_withdraw_from_sp();
        self.pause_claim_token();
        log!(
            "{} pause sys in {}",
            env::predecessor_account_id(),
            env::block_timestamp()
        );
    }

    //The external owner calls the method
    pub fn outside_open(&mut self) {
        self.assert_owner();
        self.internal_open();
        log!("Function of open by {}", env::predecessor_account_id());
    }

    pub fn outside_shutdown(&mut self) {
        self.assert_owner();
        self.internal_shutdown();
        log!("Function of shutdown by {}", env::predecessor_account_id());
    }
}
