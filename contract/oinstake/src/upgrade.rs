use near_sdk::{env, near_bindgen};

use crate::*;

#[near_bindgen]
impl OinStake {
    pub fn upgrade(
        &self,
        #[serializer(borsh)]   code: Vec<u8>,
        #[serializer(borsh)]  migrate: bool,
    ) -> Promise {
        self.assert_owner();
        let mut promise = Promise::new(env::current_account_id()).deploy_contract(code);
        if migrate {
            promise = promise.function_call(
                "migrate".into(),
                vec![],
                0,
                env::prepaid_gas() - GAS_FOR_UPGRADE_CALL - GAS_FOR_DEPLOY_CALL,
            );
        }
        promise
    }

    #[init(ignore_state)]
    #[private]
    pub fn migrate() -> Self {
        env::state_read().expect("ERR_NOT_INITIALIZED")
    }
}
