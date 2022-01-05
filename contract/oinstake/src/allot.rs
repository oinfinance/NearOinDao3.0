use near_sdk::{near_bindgen, AccountId};
use crate::*;
/*
Distribution of debt
*/
#[near_bindgen]
impl OinStake {
    // TODO [OK] Query debt allocation
    //Personal Assignment of debt and Pledge = (Debt and pledge assigned to each token in the current system - Debt pledge assigned to each token in the current individual) * Personal token
    pub(crate) fn get_account_allot(&self,account: AccountId)-> (Balance,Balance){
        let token = self.account_token.get(&account).expect(ERR_NOT_REGISTER);
        let allot = self.account_allot.get(&account).expect(ERR_NOT_REGISTER);

        let debt =U256::from(token)
            .checked_mul(
                U256::from(self.sys_allot_debt.checked_sub(allot.account_allot_debt).expect(ERR_SUB))
            ).expect(ERR_MUL)
            .checked_div(U256::from(ONE_TOKEN))
            .expect(ERR_DIV)
            .as_u128();

        let token = U256::from(token)
            .checked_mul(
                U256::from(self.sys_allot_token.checked_sub(allot.account_allot_token).expect(ERR_SUB))
            ).expect(ERR_MUL)
            .checked_div(U256::from(ONE_TOKEN))
            .expect(ERR_DIV)
            .as_u128();
        //Debt, collateral assigned to current user Token
        (debt, token)
        
    }

    //Distribution of operation
    pub(crate) fn update_account_allot(&mut self,account_id: AccountId){
        //Update [personally assigned debt, personally assigned pledge] to system value
        let (allot_debt, allot_token) = self.get_account_allot(account_id.clone());
        let token = self.account_token.get(&account_id).expect(ERR_NOT_REGISTER);
        let coin = self.account_coin.get(&account_id).expect(ERR_NOT_REGISTER);

        let after_coin = coin.checked_add(allot_debt).expect(ERR_ADD);
        let after_token = token.checked_add(allot_token).expect(ERR_ADD);

        self.account_allot.insert(
            &account_id, 
            &AccountAllot{
                account_allot_debt: self.sys_allot_debt,
                account_allot_token: self.sys_allot_token,
            }
        );
        self.account_coin.insert(&account_id, &after_coin);
        self.account_token.insert(&account_id, &after_token);       
    }

    // Be cleared to allocate, calculate each token allocation, record system value and offset
    // Debt The liquidated debt of a user
    // Pledge is the clearing pledge of the user
    // All pledges actually deducted by the Reduce_Pledge system
    pub(crate) fn liquidation_debt_allot(&mut self, debt: Balance, stake_allot_token: Balance, sys_reduce_token: Balance){
        //Debt allocated per pledge = (allocated debt + debt offset) / total pledge
        //Collateral allocated to each collateral (allocated collateral + collateral offset) / total collateral
        //Debt newly allocated by the system = debt allocated by the system + debt allocated by each pledge
        //Latest system allocated pledge = system allocated pledge + pledge allocated by each pledge
        //Collateral offset = collateral allocated to each collateral * total collateral / 10 ^ 24 - allocated collateral
        //Debt offset = debt allocated per pledge * total pledge / 10 ^ 24 - allocated debt
        if debt==0 && stake_allot_token==0 && sys_reduce_token==0 {
            log!("The stability pool has been fully allocated and the current debt pool does not need to be reallocated");
            return;
        }
        let token = self.total_token.checked_sub(sys_reduce_token).expect(ERR_SUB);

        let sys_allot_debt= (
                                U256::from(debt).checked_add(U256::from(self.debt_offset)).expect(ERR_ADD)
                            )
                            .checked_mul( U256::from(ONE_TOKEN))
                            .expect(ERR_MUL)
                            .checked_div( U256::from(token)).expect(ERR_DIV)
                            .as_u128();
        
        let sys_allot_token=(
                                U256::from(stake_allot_token).checked_add(U256::from(self.token_offset)).expect(ERR_ADD)
                            )
                            .checked_mul( U256::from(ONE_TOKEN)).expect(ERR_MUL)
                            .checked_div( U256::from(token)).expect(ERR_DIV)
                            .as_u128();
        //Calculated offset
        self.debt_offset=U256::from(debt).checked_add(U256::from(self.debt_offset)).expect(ERR_ADD)
                        .checked_sub(
                            U256::from(sys_allot_debt)
                            .checked_mul( U256::from(token)).expect(ERR_MUL)
                            .checked_div( U256::from(ONE_TOKEN)).expect(ERR_DIV)
                        ).unwrap_or(U256::from(0)).as_u128();

        self.token_offset=U256::from(stake_allot_token).checked_add(U256::from(self.token_offset)).expect(ERR_ADD)
                        .checked_sub(
                            U256::from(sys_allot_token)
                            .checked_mul( U256::from(token)).expect(ERR_MUL)
                            .checked_div( U256::from(ONE_TOKEN)).expect(ERR_DIV)
                        ).unwrap_or(U256::from(0)).as_u128();

        self.sys_allot_debt =  self.sys_allot_debt.checked_add(sys_allot_debt).expect(ERR_ADD);
        self.sys_allot_token = self.sys_allot_token.checked_add(sys_allot_token).expect(ERR_ADD);

       log!("Current per pledge assignment debt {}, current per pledge assignment token {}, debt offset {}, token offset {}",
                self.sys_allot_debt,self.sys_allot_token,self.debt_offset,self.token_offset);
    }

    //Stable pool allocation
    //debt_pledge Distribution of debt
    //token Distribute pledges
    //is_stable_allot Whether the allocation is in the stable pool
    //is_all_allot Whether all allocations are in the stable pool
    pub(crate) fn stable_stake_allot(&mut self,debt_pledge: Balance,token: Balance, allot_ratio: Balance , is_stable_allot: bool, is_all_allot: bool)-> u128  {
        let mut _token = token;
        //Allocated (stability pool) = total personal debt * ratio
        /*
            If the pledge rate is greater than or equal to 110.5%, the pledge shall be allocated 108%
            If the pledge rate is less than 110.5%
            The stable pool allocates all pledges to the empty pledge pool
            The debt is greater than the stable pool and the stable pool is allocated 108% regardless of how much is left. Allocation Pledge pool allocation
            The stabilization pool is larger than the debt pledge pool to allocate all pledges
        */
        
        //If the pledge rate is greater than or equal to 110.5%, 108% shall be allocated to the stable pool pledge pool
        if allot_ratio >= INIT_NO_LIQUIDATION_FEE_RATE {
            _token= U256::from(debt_pledge)
                                .checked_mul(U256::from(self.allot_ratio)).expect(ERR_MUL)
                                .checked_mul(U256::from(INIT_STABLE_INDEX)).expect(ERR_MUL)
                                .checked_div(U256::from(self.token_price)).expect(ERR_DIV)
                                .as_u128();                              
        }else{
            //Determine stable pool allocation
            if is_stable_allot == true {
                //If the stable pool is partially allocated
                if is_all_allot == false {
                    _token= U256::from(debt_pledge)
                                    .checked_mul(U256::from(self.allot_ratio)).expect(ERR_MUL)
                                    .checked_mul(U256::from(INIT_STABLE_INDEX)).expect(ERR_MUL)
                                    .checked_div(U256::from(self.token_price)).expect(ERR_DIV)
                                    .as_u128();
                    //If the partial stable pool is allocated according to 108%, all of it will be deducted if it is less than 108%
                    if _token > token {
                        _token = token;
                    }
                }
            }
        }

        _token

    }
   

    //Initializes the liquidator
    pub(crate) fn default_account(&mut self,account: AccountId){
        self.account_coin.insert(&account, &0u128);
        self.account_token.insert(&account, &0u128);
        self.guarantee.insert(&account, &0u128);
        
        self.account_allot.insert(
            &account,
            &AccountAllot {
                account_allot_debt: 0u128,
                account_allot_token: 0u128,
            },
        );

        self.account_stable.insert(
            &account,
            &UserStable {
                unpaid_stable_fee: 0u128,
                index: 0u128,
            },
        );
    }
}