use near_sdk::{env, near_bindgen};

use crate::*;

/*
   whitelist
*/
#[near_bindgen]
impl OinStake {
    // TODO [OK] Adding a Whitelist
    pub fn add_white(&mut self, account: AccountId) {
        self.assert_owner();
        assert!(
            !self.is_white(account.clone()),
            "The account was in whitelist yet."
        );

        self.whitelist.insert(&account);
        log!("{} add white {}", self.get_owner(), account);
    }

    // TODO [OK] Deleting a Whitelist
    pub fn remove_white(&mut self, account: AccountId) {
        self.assert_owner();
        assert!(
            self.is_white(account.clone()),
            "The account isn't in whitelist."
        );

        self.whitelist.remove(&account);
        log!("{} remove white {}", self.get_owner(), account);
    }

    // External call to determine if there is an administrator page
    pub fn is_white(&self, account: AccountId) -> bool {
        self.whitelist.contains(&account)
    }

    // num of whitelist
    pub fn white_num(&self) -> u64 {
        self.whitelist.len()
    }

    //Assertions for internal use
    pub(crate) fn assert_white(&self) {
        assert!(
            self.is_white(env::predecessor_account_id()),
            "Only white user can do it."
        );
    }
}
