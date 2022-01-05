use near_sdk::{env, log, near_bindgen, AccountId};
use near_sdk::json_types::{ValidAccountId};
use crate::*;

#[near_bindgen]
impl OinStake {
    // TODO [OK]
    pub fn get_owner(&self) -> AccountId {
        self.owner_id.clone()
    }

    // TODO [OK]
    #[payable]
    pub fn set_owner(&mut self, owner: ValidAccountId) {
        self.assert_owner();
        
        if !self.is_register(owner.clone().into()) {
            self.register_account(owner.clone().into());
        }
        self.owner_id = owner.clone().into();
        log!(
            "Owner update to {} by {}",
            self.owner_id,
            env::predecessor_account_id()
        );
    }

    pub(crate) fn assert_owner(&self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "ERR_NOT_ALLOWED"
        );
    }
   
  
}
