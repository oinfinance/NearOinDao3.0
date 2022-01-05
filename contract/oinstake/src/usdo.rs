use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider, FT_METADATA_SPEC,
};


use near_sdk::json_types::U128;
use near_sdk::{
     near_bindgen, AccountId,
    PromiseOrValue,
};
use std::convert::TryInto;

use crate::*;


#[near_bindgen]
impl OinStake {
    /* Generate money for an account */
    // TODO [OK] need to be added before use
    pub(crate) fn internal_mint(&mut self, account_id: AccountId, amount: u128) {
        self.storage_deposit(Some(account_id.as_str().try_into().unwrap()), None);
        self.token.internal_deposit(&account_id, amount.into());
    }

    /* Destroy money from an account */
    pub(crate) fn internal_burn(&mut self, account_id: AccountId, amount: u128) {
        self.token.internal_withdraw(&account_id, amount.into());
    }
    /* Query the USDO of an account */
    pub(crate) fn ft_balance(&self, account_id: AccountId)->u128 {
        self.token.ft_balance_of(account_id.as_str().try_into().unwrap()).0
    }

    // [TODO][OK] attached_deposit
    #[payable]
    pub fn register_usdo(&mut self, account: AccountId) {
        self.storage_deposit(Some(account.as_str().try_into().unwrap()), None);
    }

    // TODO[OK] Memory usage of token
    pub fn usdo_storage_usage(&self) -> u128 {
        self.token.storage_balance_bounds().min.0
    }

}

near_contract_standards::impl_fungible_token_core!(OinStake, token);
near_contract_standards::impl_fungible_token_storage!(OinStake, token);

#[near_bindgen]
impl FungibleTokenMetadataProvider for OinStake {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        FungibleTokenMetadata {
            spec: FT_METADATA_SPEC.to_string(),
            name: self.name.clone(),
            symbol: self.symbol.clone(),
            icon: Some(DATA_IMAGE_SVG_ST_USD_ICON.to_string()),
            reference: Some(self.reference.clone()),
            reference_hash: Some(self.reference_hash.clone()),
            decimals: self.decimals,
        }
    }
}
