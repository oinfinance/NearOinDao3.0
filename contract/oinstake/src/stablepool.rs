use near_sdk::json_types::U128;
use near_sdk::{env, log, near_bindgen, AccountId, Balance};

use near_sdk::collections::*;

use crate::*;

#[ext_contract(ext_stable)]
pub trait ExtStable {
    fn on_withdraw_usdo(
        &mut self,
        _receiver_id: AccountId,
        _amount: U128,
        _cha_g_bn: u64,
        _cha_g_system: InnerU256,
        _target_scale: u64,
        _target_epoch: U128,
        _before_deposits: U128,
        _before_snap: Snapshots,
    );
    fn on_withdraw_sp_reward(&mut self, _receiver_id: AccountId, _amount: U128);

    fn on_claim_dis_reward(&mut self, _claimer: AccountId, _amount: U128);
    fn on_claim_min_reward(
        &mut self,
        _claimer: AccountId,
        _token_to_claim: U128,
        _depositor_near_gain: U128,
    );
}

#[near_bindgen]
impl OinStake {
    /**provide*/
    #[payable]
    pub fn provide_to_sp(&mut self, amount: U128) {
        assert!(self.is_provide_to_sp_paused(), "{:?}", SYSTEM_PAUSE);

        let _amount = u128::from(amount);
        let provider = env::predecessor_account_id().clone();

        assert!(_amount > 0, "provide coin amount must greater than zero.");

        let compounded_usdo_deposit = self._get_compounded_usdo_deposit(provider.clone());

        let new_deposit = compounded_usdo_deposit.checked_add(_amount).expect(ERR_ADD);

        self._claim_token(provider.clone());

        //update system g
        self._update_epoch_to_scale_to_g();

        //update d_t p_t g_t s_t
        self._update_deposit_and_snapshots(provider.clone(), new_deposit);

        //do transfer
        self._send_usdo_to_stability_pool(provider.clone(), _amount);

        log!(
            "user {:?} provide {:?} usdo to stable pool , current total usdo {:?}",
            provider.clone(),
            _amount,
            self.total_usdo_deposits
        );
    }

    /**withdraw*/
    #[payable]
    pub fn withdraw_from_sp(&mut self, amount: U128) {
        let _amount = amount.into();
        let withdrawer = env::predecessor_account_id();
        let initial_deposit = self.deposits.get(&withdrawer.clone()).expect(ERR_GET);
        let compounded_usdo_deposit = self._get_compounded_usdo_deposit(withdrawer.clone());

        assert!(self.is_withdraw_from_sp_paused(), "{:?}", SYSTEM_PAUSE);
        assert!(_amount > 0, "withdraw token amount must greater than zero.");
        assert!(
            _amount <= compounded_usdo_deposit,
            "withdraw token amount too large."
        );

        let usdo_loss = initial_deposit
            .checked_sub(compounded_usdo_deposit)
            .expect(ERR_SUB); // Needed only for event log

        let new_deposit = compounded_usdo_deposit.checked_sub(_amount).expect(ERR_SUB);

        //claim min and dis reward
        self._claim_token(withdrawer.clone());

        //update system g
        self._update_epoch_to_scale_to_g();

        let before_snap = self
            .deposit_snapshots
            .get(&withdrawer.clone())
            .expect(ERR_GET);
        let before_deposits = self.deposits.get(&withdrawer.clone()).expect(ERR_GET);

        //update personal snap
        let new_scal_g = self.get_new_scale_to_g();
        self._update_deposit_and_snapshots(withdrawer.clone(), new_deposit);

        self._send_usdo_to_depositor(
            withdrawer.clone(),
            _amount,
            new_scal_g,
            before_deposits,
            before_snap,
        );

        log!(
            "{:?} withdraw {:?}usdo，current usdo of stablepool is {:?},every usdo before loss {:?}",
            withdrawer.clone(),
            _amount,
            self.total_usdo_deposits,
            usdo_loss
        );
    }

    fn _claim_dis_reward(&mut self, claimer: AccountId, depositor_near_gain: u128) {

        if depositor_near_gain > 0 {
            self.total_token_sp = self
                .total_token_sp
                .checked_sub(depositor_near_gain)
                .expect(ERR_SUB);

            //When entering the transfer, you need to clear the failed part of the transfer
            self.dis_reward.insert(&claimer.clone(), &0);

            ext_fungible_token::ft_transfer(
                claimer.clone(),
                U128(depositor_near_gain),
                None,
                &ST_NEAR,
                ONE_YOCTO_NEAR,
                GAS_FOR_FT_TRANSFER_CALL,
            )
            .then(ext_stable::on_claim_dis_reward(
                claimer.clone(),
                U128(depositor_near_gain),
                &env::current_account_id(),
                0,
                GAS_FOR_FT_TRANSFER_CALL,
            ));
        }
    }

    fn _claim_min_reward(
        &mut self,
        claimer: AccountId,
        depositor_block_gain: u128,
        depositor_near_gain: u128,
    ) -> u128 {
        log!("all depositor_block_gain {:?}", depositor_block_gain);

        let all_reward_should = depositor_block_gain;

        let token_to_claim;
        let mut unclaimed_reward = 0;

        if self.reward_sp > all_reward_should {
            token_to_claim = all_reward_should;
        } else {
            token_to_claim = self.reward_sp;
            unclaimed_reward = all_reward_should
                .checked_sub(self.reward_sp)
                .expect(ERR_SUB);
        };

        self.reward_sp = self.reward_sp.checked_sub(token_to_claim).expect(ERR_SUB);

        // self._claim_update_depositor_g_s_p(claimer.clone());

        self.min_reward.insert(&claimer.clone(), &unclaimed_reward);

        if token_to_claim > 0 {
            ext_fungible_token::ft_transfer(
                claimer.clone(),
                U128(token_to_claim),
                None,
                &ST_OIN,
                ONE_YOCTO_NEAR,
                GAS_FOR_FT_TRANSFER_CALL,
            )
            .then(ext_stable::on_claim_min_reward(
                claimer.clone(),
                U128(token_to_claim),
                U128(depositor_near_gain),
                &env::current_account_id(),
                0,
                GAS_FOR_FT_TRANSFER_CALL,
            ));
        }
        token_to_claim
    }
    /**claim dis and mint reward*/
    #[payable]
    pub fn claim_token(&mut self) {
        assert!(self.is_claim_token_paused(), "{:?}", SYSTEM_PAUSE);
        self._claim_token(env::predecessor_account_id());
    }

    fn _claim_token(&mut self, claimer: AccountId) {
        let depositor_near_gain = self.get_depositor_near_gain(&claimer.clone());
        let depositor_block_gain = self._get_near_block_gain_from_snapshots(&claimer.clone());

        //update personnal snap
        self._claim_update_depositor_g_s_p(claimer.clone());

        //min reward
        //Depositor_near_gain should be cached in the callback as well because the personal snapshot has been updated
        self._claim_min_reward(claimer.clone(), depositor_block_gain, depositor_near_gain);

        //dis reward
        //After the test, the last transfer failure will not go here, so here only needs to deal with this transfer failure depositor_near_gain
        self._claim_dis_reward(claimer.clone(), depositor_near_gain);
    }

    fn _claim_update_depositor_g_s_p(&mut self, _account: AccountId) {
        let mut depositor_snap = self.deposit_snapshots.get(&_account).expect(ERR_GET);

        //The pending value is updated with the latest pending value
        depositor_snap.g = self._get_current_g_til_last_gbn();

        log!("after claim token update g:{:?}", U256(depositor_snap.g));

        let end_snap = self._updated_depositor_s_p(depositor_snap);
        //The individual's storage should be updated before some of the individual's snapshot parameters are updated. Get_compouded uses snapshot values
        self.deposits.insert(
            &_account,
            &self._get_compounded_usdo_deposit(_account.clone()),
        );
        self.deposit_snapshots.insert(&_account, &end_snap);

    }

    //update personal snap
    fn _update_depositor_g_p(&mut self, _depositor_snap: Snapshots) -> Snapshots {
        let mut depositor_snap = _depositor_snap;

        depositor_snap.g = self
            .epoch_to_scale_to_g
            .get(&self.current_epoch)
            .expect(ERR_GET)
            .get(&self.current_scale)
            .expect(ERR_GET)
            .g_system;

        log!(
            "after provide and withdraw update g:{:?}",
            U256(depositor_snap.g)
        );

        depositor_snap
    }

    fn _updated_depositor_s_p(&mut self, _depositor_snap: Snapshots) -> Snapshots {
        let mut depositor_snap = _depositor_snap.clone();

        depositor_snap.p = self.p_system;

        depositor_snap.epoch = self.current_epoch;

        depositor_snap.s = self
            .epoch_to_scale_to_sum
            .get(&self.current_epoch)
            .expect(ERR_GET)
            .get(&self.current_scale)
            .expect(ERR_GET);

        depositor_snap.scale = self.current_scale;

        log!(
            "update  p {:?} s {:?} scale {:?} epoch {:?}",
            depositor_snap.p,
            U256(depositor_snap.s),
            depositor_snap.scale,
            depositor_snap.epoch
        );

        depositor_snap
    }

    //get increase since last system gbn
    fn _get_current_g_til_last_gbn(&mut self) -> InnerU256 {
        let current_bn = self.to_nano(env::block_timestamp());
        let system_g = self
            .epoch_to_scale_to_g
            .get(&self.current_epoch)
            .expect(ERR_GET)
            .get(&self.current_scale)
            .expect(ERR_GET);

        let g_bn = system_g.g_block_num;
        let g_system = system_g.g_system;

        let real_p = self.p_system;

        let res_g;

        if self.total_usdo_deposits == 0 {
            res_g = g_system;
        } else {
            res_g = (U256(g_system)
                .checked_add(
                    U256::from(current_bn.checked_sub(g_bn).expect(ERR_SUB))
                        .checked_mul(U256::from(self.reward_speed_sp))
                        .expect(ERR_MUL)
                        .checked_mul(U256::from(real_p))
                        .expect(ERR_MUL)
                        .checked_div(U256::from(self.total_usdo_deposits))
                        .expect(ERR_DIV),
                )
                .expect(ERR_ADD))
            .0;
        }

        log!("get currentG {:?}  from old G ", U256(res_g));

        res_g
    }

    /*liquidation operate*/
    pub(crate) fn _offset(&mut self, _dept_to_offset: u128, _coll_to_add: u128) {
        if _dept_to_offset == 0 && _coll_to_add == 0 {
            return;
        }

        assert!(
            _dept_to_offset > 0,
            "provide coin dept_to_offset must greater than zero."
        );

        assert!(
            _dept_to_offset <= self.total_usdo_deposits,
            "dept_to_offset  must lesser than total_usdo_deposits."
        );

        assert!(
            _coll_to_add > 0,
            "provide token _coll_to_add must greater than zero."
        );

        let total_usdo_cache = self.total_usdo_deposits.clone();

        if total_usdo_cache == 0 {
            return;
        }
        let (near_gain_per_unit_staked, usdo_loss_per_unit_staked) =
            self._compute_rewards_per_unit_staked(_coll_to_add, _dept_to_offset, total_usdo_cache);

        self._update_reward_sum_and_product(near_gain_per_unit_staked, usdo_loss_per_unit_staked);

        self._move_offset_coll_and_debt(_coll_to_add, _dept_to_offset);

        log! {
            "offset {:?} dept，loss {:?}usdo，gain {:?}near",_dept_to_offset,self.total_usdo_deposits,_coll_to_add
        }
    }

    //do usdo transfer
    fn _send_usdo_to_stability_pool(&mut self, sender_id: AccountId, _amount: u128) {
        self.token
            .internal_transfer(&sender_id, &env::current_account_id(), _amount.into(), None);

        let new_total_deposits = self
            .total_usdo_deposits
            .checked_add(_amount)
            .expect(ERR_ADD);

        self.total_usdo_deposits = u128::from(new_total_deposits);

        log!(
            "transfer {:?}usdo to stable pool,current usdo remain {:?}",
            _amount,
            new_total_deposits
        );
    }

    // compute rewards per usdo gain and usdo per loss
    fn _compute_rewards_per_unit_staked(
        &mut self,
        _coll_to_add: u128,
        _debt_to_offset: u128,
        _total_usdo_deposits: u128,
    ) -> (InnerU256, u128) {
        let near_gain_per_unit_staked;
        let usdo_loss_per_unit_staked;

        let near_numerator = U256::from(_coll_to_add)
            .checked_mul(U256::from(DECIMAL_PRECISION))
            .expect(ERR_MUL)
            .checked_add(U256::from(self.last_near_error_offset))
            .expect(ERR_ADD);

        if _debt_to_offset == _total_usdo_deposits {
            self.last_usdo_loss_error_offset = 0;

            usdo_loss_per_unit_staked = DECIMAL_PRECISION;
        } else {
            let usdo_loss_numerator = U256::from(_debt_to_offset)
                .checked_mul(U256::from(DECIMAL_PRECISION))
                .expect(ERR_MUL)
                .checked_sub(U256::from(self.last_usdo_loss_error_offset))
                .expect(ERR_SUB);

            usdo_loss_per_unit_staked = usdo_loss_numerator
                .checked_div(U256::from(_total_usdo_deposits))
                .expect(ERR_DIV)
                .checked_add(U256::from(1))
                .expect(ERR_ADD)
                .as_u128();

            self.last_usdo_loss_error_offset = U256::from(usdo_loss_per_unit_staked)
                .checked_mul(U256::from(_total_usdo_deposits))
                .expect(ERR_MUL)
                .checked_sub(usdo_loss_numerator)
                .expect(ERR_SUB)
                .as_u128();

            log!(
                "usdo_loss_numerator:{:?}_total_usdo_deposits:{:?}usdo_loss_per_unit_staked:{:?}last_usdo_loss_error_offset:{:?}",
                usdo_loss_numerator,_total_usdo_deposits,usdo_loss_per_unit_staked,self.last_usdo_loss_error_offset);
        }

        near_gain_per_unit_staked = near_numerator
            .checked_div(U256::from(_total_usdo_deposits))
            .expect(ERR_DIV);
        //Error offset
        self.last_near_error_offset = near_numerator
            .checked_sub(
                U256::from(near_gain_per_unit_staked)
                    .checked_mul(U256::from(_total_usdo_deposits))
                    .expect(ERR_MUL),
            )
            .expect(ERR_SUB)
            .as_u128();

        log!(
            "near_gain_per_unit_staked{:?} ,usdo_loss_per_unit_staked{:?} ",
            near_gain_per_unit_staked,
            usdo_loss_per_unit_staked
        );

        (near_gain_per_unit_staked.0, usdo_loss_per_unit_staked)
    }

    //  burn the dept and add the coll
    fn _move_offset_coll_and_debt(&mut self, _coll_to_add: Balance, _debt_to_offset: Balance) {
        self.internal_burn(env::current_account_id(), _debt_to_offset);

        self.total_token_sp = self
            .total_token_sp
            .checked_add(_coll_to_add)
            .expect(ERR_ADD);

        self._decrease_usdo(_debt_to_offset);

        log!("stablepool burn {:?} usdo", _debt_to_offset);
    }

    //decrease sys usdo
    fn _decrease_usdo(&mut self, _amount: Balance) {
        let new_total_usdo_deposits = self
            .total_usdo_deposits
            .checked_sub(_amount)
            .expect(ERR_SUB);
        self.total_usdo_deposits = new_total_usdo_deposits;

        log!(
            "current usdo of stablepool change to {:?}",
            new_total_usdo_deposits
        );
    }

    //update s and p
    fn _update_reward_sum_and_product(
        &mut self,
        _near_gain_per_unit_staked: InnerU256,
        usdo_loss_per_unit_staked: Balance,
    ) {
        assert!(usdo_loss_per_unit_staked <= DECIMAL_PRECISION);
        //update system args
        let current_p = self.p_system.clone();
        let new_p: u128;
        let new_product_factor = DECIMAL_PRECISION
            .checked_sub(usdo_loss_per_unit_staked)
            .expect(ERR_SUB);

        let scale_cached_current = self.current_scale.clone();
        let epoch_cached_current = self.current_epoch.clone();

        let current_s = self
            .epoch_to_scale_to_sum
            .get(&epoch_cached_current)
            .expect(ERR_GET)
            .get(&scale_cached_current)
            .expect(ERR_GET);

        let marginal_near_gain = U256(_near_gain_per_unit_staked)
            .checked_mul(U256::from(current_p))
            .expect(ERR_MUL)
            .checked_div(U256::from(DECIMAL_PRECISION))
            .expect(ERR_DIV);

        let new_s = U256(current_s)
            .checked_add(marginal_near_gain)
            .expect(ERR_ADD);

        /*init epoch map*/
        let mut scale_to_sum = self
            .epoch_to_scale_to_sum
            .get(&epoch_cached_current)
            .expect(ERR_GET);

        /*init S to 0*/
        scale_to_sum.insert(&scale_cached_current, &new_s.0);
        self.epoch_to_scale_to_sum
            .insert(&epoch_cached_current, &scale_to_sum);

        // If the Stability Pool was emptied, increment the epoch, and reset the scale and product P
        if new_product_factor == 0 {
            self._update_epoch_to_scale_to_g();

            self.current_epoch = epoch_cached_current.checked_add(1).expect(ERR_ADD);
            self.current_scale = 0;

            let mut scale_to_g = LookupMap::new(StorageKey::ScaleG {
                epoch: self.current_epoch,
            });

            scale_to_g.insert(
                &self.current_scale,
                &SystemG {
                    g_system: U256::zero().0,
                    g_block_num: 0,
                },
            );
            self.epoch_to_scale_to_g
                .insert(&self.current_epoch, &scale_to_g);

            let mut scale_to_sum = LookupMap::new(StorageKey::EpochSum {
                epoch: self.current_epoch.clone(),
            });

            scale_to_sum.insert(&0, &U256::zero().0);
            self.epoch_to_scale_to_sum
                .insert(&self.current_epoch, &scale_to_sum);

            new_p = DECIMAL_PRECISION;
        } else if U256::from(current_p)
            .checked_mul(U256::from(new_product_factor))
            .expect(ERR_MUL)
            .checked_div(U256::from(DECIMAL_PRECISION))
            .expect(ERR_DIV)
            .as_u128()
            < SCALE_FACTOR
        {
            new_p = U256::from(current_p)
                .checked_mul(U256::from(new_product_factor))
                .expect(ERR_DIV)
                .checked_mul(U256::from(SCALE_FACTOR))
                .expect(ERR_MUL)
                .checked_div(U256::from(DECIMAL_PRECISION))
                .expect(ERR_DIV)
                .as_u128();

            //here going to  clear last scale's G
            self._update_epoch_to_scale_to_g();
            self.current_scale = scale_cached_current.checked_add(1).expect(ERR_ADD);

            //update G
            let mut scale_to_g = self
                .epoch_to_scale_to_g
                .get(&self.current_epoch)
                .expect(ERR_GET);

            scale_to_g.insert(
                &self.current_scale,
                &SystemG {
                    g_system: U256::zero().0,
                    g_block_num: self.to_nano(env::block_timestamp()),
                },
            );

            self.epoch_to_scale_to_g
                .insert(&self.current_epoch, &scale_to_g);

            //update sum
            let mut scale_to_sum = self
                .epoch_to_scale_to_sum
                .get(&self.current_epoch)
                .expect(ERR_GET);

            scale_to_sum.insert(&self.current_scale, &U256::zero().0);

            self.epoch_to_scale_to_sum
                .insert(&self.current_epoch, &scale_to_sum);
        } else {
            new_p = U256::from(current_p)
                .checked_mul(U256::from(U256::from(new_product_factor)))
                .expect(ERR_MUL)
                .checked_div(U256::from(DECIMAL_PRECISION))
                .expect(ERR_DIV)
                .as_u128();
        }

        self.p_system = new_p;

        log!("update epoch {:?} scale {:?} and curS {:?} newS {:?} current_p {:?} update P to {:?}_near_gain_per_unit_staked{:?}",self.current_epoch,self.current_scale,U256(current_s),new_s,current_p,new_p,U256(_near_gain_per_unit_staked));
    }

    //withdraw usd
    fn _send_usdo_to_depositor(
        &mut self,
        _receiver_id: AccountId,
        _amount: u128,

        _new_system_g: SystemG,
        before_deposits: u128,
        before_snap: Snapshots,
    ) {
        //The system value needs to be modified
        let unchange_system_g = self
            .epoch_to_scale_to_g
            .get(&self.current_epoch)
            .expect(ERR_GET)
            .get(&self.current_scale)
            .expect(ERR_GET);

        let before_g_bn = unchange_system_g.g_block_num;
        let before_g_system = unchange_system_g.g_system;
        let cha_g_bn = _new_system_g
            .g_block_num
            .checked_sub(before_g_bn)
            .expect(ERR_SUB);
        let cha_g_system = U256(_new_system_g.g_system)
            .checked_sub(U256(before_g_system))
            .expect(ERR_SUB);

        let mut scale_to_g = self
            .epoch_to_scale_to_g
            .get(&self.current_epoch)
            .expect(ERR_GET);

        scale_to_g.insert(&self.current_scale, &_new_system_g);
        self.epoch_to_scale_to_g
            .insert(&self.current_epoch, &scale_to_g);

        let new_total_deposits = self
            .total_usdo_deposits
            .checked_sub(_amount)
            .expect(ERR_SUB);
        self.total_usdo_deposits = new_total_deposits;

        log!(
            "normal transfer:self.total_usdo_deposits{:?}g_block_num{:?}g_system{:?}",
            self.total_usdo_deposits,
            _new_system_g.g_block_num,
            U256(_new_system_g.g_system.into())
        );

        ext_fungible_token::ft_transfer(
            _receiver_id.clone(),
            U128(_amount.into()),
            None,
            &env::current_account_id(),
            ONE_YOCTO_NEAR,
            GAS_FOR_FT_TRANSFER_CALL,
        )
        .then(ext_stable::on_withdraw_usdo(
            _receiver_id.clone(),
            U128(_amount.into()),
            cha_g_bn.into(),
            cha_g_system.0,
            self.current_scale,
            U128(self.current_epoch),
            U128(before_deposits),
            before_snap,
            &env::current_account_id(),
            0,
            GAS_FOR_FT_TRANSFER_CALL,
        ));

        log!("send {:?} usdo to {:?}", _amount, _receiver_id);
    }

    // update personal snap
    fn _update_deposit_and_snapshots(&mut self, _account: AccountId, _amount: u128) {
        let depositor_snap = self.deposit_snapshots.get(&_account).expect(ERR_GET);

        //What's updated here is the system's G assignment
        let handled_snap = self._update_depositor_g_p(depositor_snap);

        let end_snap = self._updated_depositor_s_p(handled_snap);

        self.deposit_snapshots.insert(&_account, &end_snap);

        self.deposits.insert(&_account, &_amount);
    }

    fn get_new_scale_to_g(&mut self) -> SystemG {
        let last_total = self.total_usdo_deposits;
        let current_p = self.p_system;

        let mut system_g = self
            .epoch_to_scale_to_g
            .get(&self.current_epoch)
            .expect(ERR_GET)
            .get(&self.current_scale)
            .expect(ERR_GET);
        let current_g = system_g.g_system;
        let current_g_bn = system_g.g_block_num;
        let current_bn = self.to_nano(env::block_timestamp());

        if current_g_bn == 0 {
            system_g.g_block_num = current_bn;
            system_g.g_system = U256::zero().0;

            return system_g;
        }

        let new_g;
        if last_total == 0 {
            new_g = U256(current_g).0;
        } else {
            new_g = (U256(current_g)
                .checked_add(
                    U256::from(
                        u128::from(current_bn.checked_sub(current_g_bn).expect(ERR_SUB))
                            .checked_mul(self.reward_speed_sp)
                            .expect(ERR_MUL),
                    )
                    .checked_mul(U256::from(current_p))
                    .expect(ERR_MUL)
                    .checked_div(U256::from(self.total_usdo_deposits))
                    .expect(ERR_DIV),
                )
                .expect(ERR_ADD))
            .0;
        }

        system_g.g_block_num = current_bn;
        system_g.g_system = new_g;

        log!(
            "update g for mint gain : last_total {:?} current_epoch {:?} current_scale {:?} update g_system {:?} and gbn {:?} current_bn {:?} current_p {:?} current_g {:?}",
            last_total,self.current_epoch,self.current_scale,U256(system_g.g_system),current_g_bn,current_bn,current_p,U256(current_g)
        );

        system_g
    }

    pub(crate) fn _update_epoch_to_scale_to_g(&mut self) {
        let new_system_g = self.get_new_scale_to_g();
        let mut new_scale_to_g = self
            .epoch_to_scale_to_g
            .get(&self.current_epoch)
            .expect(ERR_GET);

        new_scale_to_g.insert(&self.current_scale, &new_system_g);
        self.epoch_to_scale_to_g
            .insert(&self.current_epoch, &new_scale_to_g);
    }

    pub(crate) fn _get_compounded_usdo_deposit(&self, _account: AccountId) -> u128 {
        if let Some(initial_deposit) = self.deposits.get(&_account) {
            let snap = self
                .deposit_snapshots
                .get(&_account)
                .expect(ERR_NOT_REGISTER);
            self._get_compounded_stake_from_snapshots(initial_deposit, snap)
        } else {
            0
        }
    }

    fn _get_compounded_stake_from_snapshots(
        &self,
        initial_deposit: Balance,
        snap: Snapshots,
    ) -> u128 {
        let snapshot_p = snap.p;
        let scale_snapshot = snap.scale;
        let epoch_snapshot = snap.epoch;
        let compounded_stake;

        if epoch_snapshot < self.current_epoch {
            return 0;
        }

        let scale_diff = self.current_scale.checked_sub(scale_snapshot);
        match scale_diff {
            Some(0) => {
                compounded_stake = U256::from(initial_deposit)
                    .checked_mul(U256::from(self.p_system))
                    .expect(ERR_MUL)
                    .checked_div(U256::from(snapshot_p))
                    .expect(ERR_DIV)
                    .as_u128()
            }
            Some(1) => {
                compounded_stake = U256::from(initial_deposit)
                    .checked_mul(U256::from(self.p_system))
                    .expect(ERR_MUL)
                    .checked_div(U256::from(snapshot_p))
                    .expect(ERR_DIV)
                    .checked_div(U256::from(SCALE_FACTOR))
                    .expect(ERR_DIV)
                    .as_u128()
            }
            _ => compounded_stake = 0,
        }
        log!(
            "initial_deposit{:?}self.p_system{:?}snapshot_p{:?}",
            initial_deposit,
            self.p_system,
            snapshot_p
        );

        compounded_stake
    }

    // Todo is not currently used in the front end and can be considered in the future
    pub fn get_depositor_near_gain(&self, _account: &AccountId) -> u128 {
        let initial_deposit = self.deposits.get(&_account).expect(ERR_NOT_REGISTER);
        let unclaimed = self.dis_reward.get(&_account).expect(ERR_NOT_REGISTER);

        if initial_deposit == 0 {
            return 0;
        }

        let snapshots = self.deposit_snapshots.get(&_account).expect(ERR_GET);

        self._get_near_gain_from_snapshots(initial_deposit, &snapshots)
            .checked_add(unclaimed)
            .expect(ERR_GET)
    }

    fn _get_near_gain_from_snapshots(
        &self,
        _initial_deposit: Balance,
        _snapshots: &Snapshots,
    ) -> u128 {
        let epoch_snapshot = _snapshots.epoch;
        let scale_snapshot = _snapshots.scale;
        let s_snapshot = _snapshots.s;
        let p_snapshot = _snapshots.p;

        let first_portion = U256(
            self.epoch_to_scale_to_sum
                .get(&epoch_snapshot)
                .expect(ERR_GET)
                .get(&scale_snapshot)
                .expect(ERR_GET),
        )
        .checked_sub(U256(s_snapshot))
        .expect(ERR_SUB);

        let second_portion;

        if let Some(next) = self
            .epoch_to_scale_to_sum
            .get(&epoch_snapshot)
            .expect(ERR_GET)
            .get(&(scale_snapshot + 1))
        {
            second_portion = U256(next)
                .checked_div(U256::from(SCALE_FACTOR))
                .expect(ERR_DIV);
        } else {
            second_portion = U256::zero();
        }

        let near_gain = U256::from(_initial_deposit)
            .checked_mul(U256::from(
                first_portion.checked_add(second_portion).expect(ERR_ADD),
            ))
            .expect(ERR_MUL)
            .checked_div(U256::from(p_snapshot))
            .expect(ERR_DIV)
            .as_u128();

        log!("scale_snapshot is {:?} system scale is {:?} first_portion{:?},second_portion{:?}_initial_deposit{:?}p_snapshot{:?}", scale_snapshot,self.current_scale,
            first_portion,second_portion,_initial_deposit,p_snapshot);

        near_gain
    }

    //  get mint near
    fn _get_scale_block_gain(&self, _account: &AccountId, _scale: &u64) -> u128 {
        let person_snap = self.deposit_snapshots.get(&_account).expect(ERR_GET);

        let system_g = self
            .epoch_to_scale_to_g
            .get(&person_snap.epoch)
            .expect(ERR_GET)
            .get(&_scale)
            .expect(ERR_GET);

        let g_bn = u128::from(system_g.g_block_num);
        let g_system = system_g.g_system;
        let p_system = self.p_system;

        let d_t = self.deposits.get(&_account).expect(ERR_GET);
        let g_t = person_snap.g;
        let p_t = person_snap.p;

        let mut first_part = U256::zero();
        let mut third_part = U256::zero();
        let second_part = U256(g_system)
            .checked_mul(U256::from(d_t))
            .expect(ERR_MUL)
            .checked_div(U256::from(p_t))
            .expect(ERR_DIV);

        let diff = self
            .current_scale
            .checked_sub(_scale.clone())
            .expect(ERR_SUB);

        if _scale.checked_sub(person_snap.scale).expect(ERR_SUB) == 0 {
            third_part = U256(g_t)
                .checked_mul(U256::from(d_t))
                .expect(ERR_MUL)
                .checked_div(U256::from(p_t))
                .expect(ERR_DIV);
        }

        if diff == 0 {
            //cur user experience one scale
            let cur_block_num = u128::from(self.to_nano(env::block_timestamp()));

            log!(
                "cur_block_num{:?}g_bn{:?}self.reward_speed_sp{:?}d_t{:?}",
                cur_block_num,
                g_bn,
                self.reward_speed_sp,
                d_t
            );
            first_part = U256::from(
                (cur_block_num.checked_sub(g_bn).expect(ERR_SUB))
                    .checked_mul(self.reward_speed_sp)
                    .expect(ERR_MUL),
            )
            .checked_mul(U256::from(d_t))
            .expect(ERR_MUL)
            .checked_mul(U256::from(p_system))
            .expect(ERR_MUL)
            .checked_div(U256::from(self.total_usdo_deposits))
            .expect(ERR_DIV)
            .checked_div(U256::from(p_t))
            .expect(ERR_DIV);
        }

        let res = (first_part
            .checked_add(second_part)
            .expect(ERR_ADD)
            .checked_sub(third_part)
            .expect(ERR_SUB))
        .as_u128();

        log!(
            "current_epoch{:?} target_scale{:?} diff is {:?} first_part{:?}second_part{:?}third_part{:?}",
            self.current_epoch,
            _scale,
            diff,
            first_part,
            second_part,
            third_part
        );

        res
    }

    //  get mint near
    pub(crate) fn _get_near_block_gain_from_snapshots(&self, _account: &AccountId) -> u128 {
        let person_snap = self.deposit_snapshots.get(&_account).expect(ERR_GET);

        let g_t = person_snap.g;
        let p_t = person_snap.p;

        let d_t = self.deposits.get(&_account).expect(ERR_GET);
        let un_claimed_reward = self.min_reward.get(&_account).expect(ERR_GET);

        if self.total_usdo_deposits == 0 {
            if person_snap.epoch == self.current_epoch {
                log!("un_claimed_reward  is {:?}", un_claimed_reward);

                return un_claimed_reward;
            }
        }

        if person_snap.epoch < self.current_epoch {
            let cur_epoch = self
                .epoch_to_scale_to_g
                .get(&person_snap.epoch)
                .expect(ERR_GET);

            let first_g_system = cur_epoch.get(&person_snap.scale).expect(ERR_GET).g_system;

            let first_scale = U256(first_g_system)
                .checked_sub(U256(g_t))
                .expect(ERR_SUB)
                .checked_mul(U256::from(d_t))
                .expect(ERR_MUL)
                .checked_div(U256::from(p_t))
                .expect(ERR_DIV)
                .as_u128();

            log!(
                "first_g_system is {:?}g_t is {:?} d_t is {:?} p_t is {:?} first_scale {:?}",
                U256(first_g_system),
                U256(g_t),
                d_t,
                p_t,
                first_scale
            );
            let mut second_scale = 0;
            if cur_epoch.contains_key(&(person_snap.scale + 1)) {
                let second_g_system = cur_epoch
                    .get(&(person_snap.scale + 1))
                    .expect(ERR_GET)
                    .g_system;

                second_scale = U256(second_g_system)
                    .checked_mul(U256::from(d_t))
                    .expect(ERR_MUL)
                    .checked_div(U256::from(p_t))
                    .expect(ERR_DIV)
                    .checked_div(U256::from(SCALE_FACTOR))
                    .expect(ERR_DIV)
                    .as_u128();
                log!(
                    "second_g_system is {:?} d_t is {:?} p_t is {:?} second_scale {:?}",
                    U256(second_g_system),
                    d_t,
                    p_t,
                    second_scale
                );
            }

            return first_scale
                .checked_add(second_scale)
                .expect(ERR_ADD)
                .checked_add(un_claimed_reward)
                .expect(ERR_ADD);
        }

        let one = self._get_scale_block_gain(_account, &person_snap.scale);
        let mut two = 0;

        if person_snap.scale < self.current_scale {
            two = self
                ._get_scale_block_gain(_account, &(person_snap.scale + 1))
                .checked_div(SCALE_FACTOR)
                .expect(ERR_DIV);
        }

        log!("system current_epoch {:?} personepoch {:?} personscale {:?} pt{:?} res consist of one {:?},two {:?},un_claimed_reward {:?}",
         self.current_epoch,person_snap.epoch,person_snap.scale,p_t,one,two,un_claimed_reward);

        one.checked_add(two)
            .expect(ERR_ADD)
            .checked_add(un_claimed_reward)
            .expect(ERR_ADD)
    }

    pub fn withdraw_sp_reward(&mut self, amount: U128) {
        self.assert_owner();
        let _amount = u128::from(amount);
        assert!(
            self.reward_sp >= _amount && _amount > 0,
            "amount should lower than total reward"
        );

        let _receiver_id = env::predecessor_account_id();

        self.reward_sp = self.reward_sp.checked_sub(_amount).expect(ERR_SUB);

        log!("withdraw_before{:?}", self.reward_sp);
        ext_fungible_token::ft_transfer(
            _receiver_id.clone(),
            U128(amount.into()),
            None,
            &ST_OIN,
            ONE_YOCTO_NEAR,
            GAS_FOR_FT_TRANSFER_CALL,
        )
        .then(ext_stable::on_withdraw_sp_reward(
            _receiver_id.clone(),
            U128(amount.into()),
            &env::current_account_id(),
            0,
            GAS_FOR_FT_TRANSFER_CALL,
        ));
    }

    pub(crate) fn inject_sp_reward(&mut self, _amount: U128, sender_id: ValidAccountId) {
        self.reward_sp = self
            .reward_sp
            .checked_add(u128::from(_amount))
            .expect(ERR_ADD);

        log!(
            "{:?} add sp_reward  {:?} cur reward_sp {:?}",
            sender_id,
            u128::from(_amount),
            self.reward_sp
        );
    }
}
