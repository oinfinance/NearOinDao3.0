use near_sdk::{env, near_bindgen, Promise, PromiseOrValue};
use std::collections::HashSet;

use crate::*;

#[ext_contract(ext_multisign)]
pub trait ExtMultisign {
    fn on_function_call(&mut self, request_id: RequestId);
}
#[near_bindgen]
impl OinStake {
    /// Add request for multisig and confirm with the pk that added.
    pub fn add_request(&mut self, inner_request: MultiSigRequest) -> RequestId {
        self.assert_mul_white();

        let request_id = self.add_request_only(inner_request);
        self._confirm(request_id);

        request_id
    }

    pub fn delete_request(&mut self, request_id: RequestId) {
        self.assert_owner();
        self.assert_valid_request(request_id);

        self.remove_request(request_id);
    }

    //The front end is also called here
    pub fn confirm(&mut self, request_id: RequestId) {
        self.assert_mul_white();
        self.assert_valid_request(request_id);

        self._confirm(request_id);
       
    }
    
    fn _confirm(&mut self, request_id: RequestId) {
        let mut confirmations = self.confirmations.get(&request_id).unwrap();
        let mut request = self.requests.get(&request_id).unwrap();

        assert!(
            !confirmations.contains(&env::signer_account_pk()),
            "Already confirmed this request with this key"
        );

        assert!(
            request.confirmed_timestamp ==0,
            "request has confirmed"
        );


        confirmations.insert(env::signer_account_pk());
        self.confirmations.insert(&request_id, &confirmations);

        if self.is_num_enough(request_id) {
            
            request.confirmed_timestamp = env::block_timestamp();
            request.mul_white_num =self.mul_white_num();
            self.requests.insert(&request_id, &request);
        }
    }
    


    pub fn exe_request(&mut self, request_id: RequestId) -> PromiseOrValue<bool> {
        self.assert_mul_white();

        assert!(
            self.is_executable(request_id),
            "request can not be exe , time or vote too short!"
        );
        let mut request_one = self.requests.get(&request_id).expect(ERR_GET);

        let inner_request = request_one.clone().request;
        request_one.is_executed = true;
        self.requests.insert(&request_id, &request_one);

        self.execute_request(inner_request, request_id)
    }

    fn add_request_only(&mut self, inner_request: MultiSigRequest) -> RequestId {

        let num_requests = self
            .num_requests_pk
            .get(&env::signer_account_pk())
            .unwrap_or(0)
            + 1;

        self.num_requests_pk
            .insert(&env::signer_account_pk(), &num_requests);

        let request_added = MultiSigRequestWithSigner {
            signer_pk: env::signer_account_pk(),
            added_timestamp: env::block_timestamp(),
            confirmed_timestamp: 0,
            request: inner_request,
            is_executed: false,
            cool_down: self.request_cooldown,
            mul_white_num: 0,
            num_confirm_ratio: self.num_confirm_ratio,
        };

        self.requests.insert(&self.request_nonce, &request_added);
        let confirmations = HashSet::new();
        self.confirmations
            .insert(&self.request_nonce, &confirmations);
        self.request_nonce += 1;
        self.request_nonce - 1
    }

    fn execute_request(
        &mut self,
        inner_request: MultiSigRequest,
        request_id: RequestId,
    ) -> PromiseOrValue<bool> {
        let mut promise = Promise::new(env::current_account_id());
        for action in inner_request.actions {
            promise = match action {
                MultiSigRequestAction::FunctionCall {
                    method_name,
                    args,
                    deposit,
                    gas,
                } => promise
                    .function_call(
                        method_name.into_bytes(),
                        args.into(),
                        deposit.into(),
                        gas.into(),
                    )
                    .then(ext_multisign::on_function_call(
                        request_id,
                        &env::current_account_id(),
                        0,
                        GAS_FOR_FT_TRANSFER_CALL,
                    )),
            };
        }
        promise.into()
    }

    pub(crate) fn assert_mul_white(&self) {
        assert!(
            self.is_mul_white(env::predecessor_account_id()),
            "Only multsign white user can do it."
        );
    }

    fn remove_request(&mut self, request_id: RequestId) -> MultiSigRequest {
        self.confirmations.remove(&request_id);
        let request_with_signer = self
            .requests
            .remove(&request_id)
            .expect("Failed to remove existing element");

        let original_signer_pk = request_with_signer.signer_pk;
        let mut num_requests = self.num_requests_pk.get(&original_signer_pk).unwrap_or(0);
        if num_requests > 0 {
            num_requests = num_requests - 1;
        }
        self.num_requests_pk
            .insert(&original_signer_pk, &num_requests);

        request_with_signer.request
    }

    // Prevents access to calling requests and make sure request_id is valid - used in delete and confirm
    fn assert_valid_request(&mut self, request_id: RequestId) {
        // request must exist and have
        assert!(
            self.requests.get(&request_id).is_some(),
            "No such request: either wrong number or already confirmed"
        );
        assert!(
            self.confirmations.get(&request_id).is_some(),
            "Internal error: confirmations mismatch requests"
        );
    }

    pub(crate) fn is_num_enough(&self, request_id: RequestId) -> bool {
        let request = self.requests.get(&request_id).unwrap();
        let confirmations = self.confirmations.get(&request_id).unwrap();
        let  num_confirmrations;


        //If no vote is passed, the latest mul_num is counted; otherwise, the mul_num at the passed point in time prevails
        if request.confirmed_timestamp==0 {
             num_confirmrations = request.num_confirm_ratio.checked_mul(self.mul_white_num()).expect(ERR_MUL);

        }else{
             num_confirmrations = request.num_confirm_ratio.checked_mul(request.mul_white_num).expect(ERR_MUL);
        }

        (confirmations.len() as u64).checked_mul(ONE_FOR_NUM_RATIO).expect(ERR_MUL) >= num_confirmrations
    }

    fn is_executable(&self, request_id: RequestId) -> bool {
        let request = self.requests.get(&request_id).unwrap();

        (self.is_num_enough(request_id)) && !request.is_executed
            && (env::block_timestamp().checked_sub(request.confirmed_timestamp)).expect(ERR_SUB) >= request.cool_down
    }
}
