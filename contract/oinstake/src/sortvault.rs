use near_sdk::json_types::ValidAccountId;
use near_sdk::{log, near_bindgen};

use crate::*;

#[near_bindgen]
impl OinStake {
    /*
     *insert operation
     */
    pub(crate) fn _vault_init(&mut self, _id: ValidAccountId) {
        let cur_index = self.unsorted_vaults.len();
        self.unsorted_vaults.insert(cur_index, _id.clone());
        self.account_to_index.insert(&_id, &cur_index);

        log!("insert{} index is {}", _id, cur_index);
    }

    /*
     * remove operation
     */
    pub(crate) fn _vault_remove(&mut self, _id: ValidAccountId) {
        let index = self.account_to_index.get(&_id).expect(ERR_GET);

        let change_account = self
            .unsorted_vaults
            .get(self.unsorted_vaults.len().checked_sub(1).expect(ERR_SUB))
            .expect(ERR_GET);

        self.account_to_index.insert(&change_account, &index);
        self.unsorted_vaults.swap_remove(index);

        log!("going to remove{} index{}", _id, index);
    }

    /*
     *list of liqutation
     */
    pub fn list_liqutations(&self) -> Vec<ValidAccountId> {
        self.unsorted_vaults.clone()
    }

    /*
     * return token dept user_ratio
     */
    pub fn get_liqutation_detail(&self, account: AccountId) -> (U128, U128, U128) {
        let account_collet = self.get_debt(account.clone());
        let user_ratio = self.get_user_ratio(account);
        (
            U128(account_collet.1),
            U128(account_collet.0),
            user_ratio
        )
    }
}
