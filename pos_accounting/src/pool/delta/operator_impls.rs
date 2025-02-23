// Copyright (c) 2022 RBB S.r.l
// opensource@mintlayer.org
// SPDX-License-Identifier: MIT
// Licensed under the MIT License;
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// https://github.com/mintlayer/mintlayer-core/blob/master/LICENSE
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::BTreeMap;

use accounting::{combine_amount_delta, combine_data_with_delta, DataDelta};
use common::{chain::OutPoint, primitives::Amount};
use crypto::key::PublicKey;

use crate::{
    error::Error,
    pool::{
        delegation::DelegationData,
        helpers::{make_delegation_id, make_pool_id},
        operations::{
            CreateDelegationIdUndo, CreatePoolUndo, DecommissionPoolUndo, DelegateStakingUndo,
            DelegationDataUndo, PoSAccountingOperatorRead, PoSAccountingOperatorWrite,
            PoSAccountingUndo, PoolDataUndo, SpendFromShareUndo,
        },
        pool_data::PoolData,
    },
    DelegationId, PoolId,
};

use super::{sum_maps, PoSAccountingDelta};

impl<'a> PoSAccountingOperatorWrite for PoSAccountingDelta<'a> {
    fn create_pool(
        &mut self,
        input0_outpoint: &OutPoint,
        pledge_amount: Amount,
        decommission_key: PublicKey,
    ) -> Result<(PoolId, PoSAccountingUndo), Error> {
        let pool_id = make_pool_id(input0_outpoint);

        if self.get_pool_balance(pool_id)?.is_some() {
            // This should never happen since it's based on an unspent input
            return Err(Error::InvariantErrorPoolBalanceAlreadyExists);
        }

        if self.get_pool_data(pool_id)?.is_some() {
            // This should never happen since it's based on an unspent input
            return Err(Error::InvariantErrorPoolDataAlreadyExists);
        }

        self.data.pool_balances.add_unsigned(pool_id, pledge_amount)?;
        let undo_data = self.data.pool_data.merge_delta_data_element(
            pool_id,
            DataDelta::Create(Box::new(PoolData::new(decommission_key, pledge_amount))),
        )?;

        Ok((
            pool_id,
            PoSAccountingUndo::CreatePool(CreatePoolUndo {
                pool_id,
                data_undo: PoolDataUndo::DataDelta((pledge_amount, undo_data)),
            }),
        ))
    }

    fn decommission_pool(&mut self, pool_id: PoolId) -> Result<PoSAccountingUndo, Error> {
        let last_amount = self
            .get_pool_balance(pool_id)?
            .ok_or(Error::AttemptedDecommissionNonexistingPoolBalance)?;

        self.get_pool_data(pool_id)?
            .ok_or(Error::AttemptedDecommissionNonexistingPoolData)?;

        self.data.pool_balances.sub_unsigned(pool_id, last_amount)?;
        let data_undo = self.data.pool_data.merge_delta_data_element(pool_id, DataDelta::Delete)?;

        Ok(PoSAccountingUndo::DecommissionPool(DecommissionPoolUndo {
            pool_id,
            data_undo: PoolDataUndo::DataDelta((last_amount, data_undo)),
        }))
    }

    fn create_delegation_id(
        &mut self,
        target_pool: PoolId,
        spend_key: PublicKey,
        input0_outpoint: &OutPoint,
    ) -> Result<(DelegationId, PoSAccountingUndo), Error> {
        if !self.pool_exists(target_pool)? {
            return Err(Error::DelegationCreationFailedPoolDoesNotExist);
        }

        let delegation_id = make_delegation_id(input0_outpoint);

        if self.get_delegation_id_data(delegation_id)?.is_some() {
            // This should never happen since it's based on an unspent input
            return Err(Error::InvariantErrorDelegationCreationFailedIdAlreadyExists);
        }

        let delegation_data = DelegationData::new(target_pool, spend_key);

        let data_undo = self.data.delegation_data.merge_delta_data_element(
            delegation_id,
            DataDelta::Create(Box::new(delegation_data)),
        )?;

        Ok((
            delegation_id,
            PoSAccountingUndo::CreateDelegationId(CreateDelegationIdUndo {
                delegation_id,
                data_undo: DelegationDataUndo::DataDelta(data_undo),
            }),
        ))
    }

    fn delegate_staking(
        &mut self,
        delegation_target: DelegationId,
        amount_to_delegate: Amount,
    ) -> Result<PoSAccountingUndo, Error> {
        let pool_id = *self
            .get_delegation_id_data(delegation_target)?
            .ok_or(Error::DelegationCreationFailedPoolDoesNotExist)?
            .source_pool();

        self.add_to_delegation_balance(delegation_target, amount_to_delegate)?;

        self.add_balance_to_pool(pool_id, amount_to_delegate)?;

        self.add_delegation_to_pool_share(pool_id, delegation_target, amount_to_delegate)?;

        Ok(PoSAccountingUndo::DelegateStaking(DelegateStakingUndo {
            delegation_target,
            amount_to_delegate,
        }))
    }

    fn spend_share_from_delegation_id(
        &mut self,
        delegation_id: DelegationId,
        amount: Amount,
    ) -> Result<PoSAccountingUndo, Error> {
        let pool_id = *self
            .get_delegation_id_data(delegation_id)?
            .ok_or(Error::InvariantErrorDelegationUndoFailedDataNotFound)?
            .source_pool();

        self.sub_delegation_from_pool_share(pool_id, delegation_id, amount)?;

        self.sub_balance_from_pool(pool_id, amount)?;

        self.sub_from_delegation_balance(delegation_id, amount)?;

        Ok(PoSAccountingUndo::SpendFromShare(SpendFromShareUndo {
            delegation_id,
            amount,
        }))
    }

    fn undo(&mut self, undo_data: PoSAccountingUndo) -> Result<(), Error> {
        match undo_data {
            PoSAccountingUndo::CreatePool(undo) => self.undo_create_pool(undo),
            PoSAccountingUndo::DecommissionPool(undo) => self.undo_decommission_pool(undo),
            PoSAccountingUndo::CreateDelegationId(undo) => self.undo_create_delegation_id(undo),
            PoSAccountingUndo::DelegateStaking(undo) => self.undo_delegate_staking(undo),
            PoSAccountingUndo::SpendFromShare(undo) => {
                self.undo_spend_share_from_delegation_id(undo)
            }
        }
    }
}

impl<'a> PoSAccountingDelta<'a> {
    fn undo_create_pool(&mut self, undo: CreatePoolUndo) -> Result<(), Error> {
        let (pledge_amount, undo_data) = match undo.data_undo {
            PoolDataUndo::DataDelta(v) => v,
            PoolDataUndo::Data(_) => unreachable!("incompatible PoolDataUndo supplied"),
        };
        let amount = self.get_pool_balance(undo.pool_id)?;

        match amount {
            Some(amount) => {
                if amount != pledge_amount {
                    return Err(Error::InvariantErrorPoolCreationReversalFailedAmountChanged);
                }
            }
            None => return Err(Error::InvariantErrorPoolCreationReversalFailedBalanceNotFound),
        }

        self.data.pool_balances.sub_unsigned(undo.pool_id, pledge_amount)?;

        self.get_pool_data(undo.pool_id)?
            .ok_or(Error::InvariantErrorPoolCreationReversalFailedDataNotFound)?;

        self.data.pool_data.undo_merge_delta_data_element(undo.pool_id, undo_data)?;

        Ok(())
    }

    fn undo_decommission_pool(&mut self, undo: DecommissionPoolUndo) -> Result<(), Error> {
        let (last_amount, undo_data) = match undo.data_undo {
            PoolDataUndo::DataDelta(v) => v,
            PoolDataUndo::Data(_) => unreachable!("incompatible PoolDataUndo supplied"),
        };

        if self.get_pool_balance(undo.pool_id)?.is_some() {
            return Err(Error::InvariantErrorDecommissionUndoFailedPoolBalanceAlreadyExists);
        }

        if self.get_pool_data(undo.pool_id)?.is_some() {
            return Err(Error::InvariantErrorDecommissionUndoFailedPoolDataAlreadyExists);
        }

        self.data.pool_balances.add_unsigned(undo.pool_id, last_amount)?;
        self.data.pool_data.undo_merge_delta_data_element(undo.pool_id, undo_data)?;

        Ok(())
    }

    fn undo_create_delegation_id(&mut self, undo: CreateDelegationIdUndo) -> Result<(), Error> {
        let undo_data = match undo.data_undo {
            DelegationDataUndo::DataDelta(v) => v,
            DelegationDataUndo::Data(_) => unreachable!("incompatible DelegationDataUndo supplied"),
        };

        self.get_delegation_id_data(undo.delegation_id)?
            .ok_or(Error::InvariantErrorDelegationIdUndoFailedNotFound)?;

        self.data
            .delegation_data
            .undo_merge_delta_data_element(undo.delegation_id, undo_data)?;

        Ok(())
    }

    fn undo_delegate_staking(&mut self, undo_data: DelegateStakingUndo) -> Result<(), Error> {
        let pool_id = *self
            .get_delegation_id_data(undo_data.delegation_target)?
            .ok_or(Error::InvariantErrorDelegationUndoFailedDataNotFound)?
            .source_pool();

        self.sub_delegation_from_pool_share(
            pool_id,
            undo_data.delegation_target,
            undo_data.amount_to_delegate,
        )?;

        self.sub_balance_from_pool(pool_id, undo_data.amount_to_delegate)?;

        self.sub_from_delegation_balance(
            undo_data.delegation_target,
            undo_data.amount_to_delegate,
        )?;

        Ok(())
    }

    fn undo_spend_share_from_delegation_id(
        &mut self,
        undo_data: SpendFromShareUndo,
    ) -> Result<(), Error> {
        let pool_id = *self
            .get_delegation_id_data(undo_data.delegation_id)?
            .ok_or(Error::DelegationCreationFailedPoolDoesNotExist)?
            .source_pool();

        self.add_to_delegation_balance(undo_data.delegation_id, undo_data.amount)?;

        self.add_balance_to_pool(pool_id, undo_data.amount)?;

        self.add_delegation_to_pool_share(pool_id, undo_data.delegation_id, undo_data.amount)?;

        Ok(())
    }
}

impl<'a> PoSAccountingOperatorRead for PoSAccountingDelta<'a> {
    fn pool_exists(&self, pool_id: PoolId) -> Result<bool, Error> {
        Ok(self
            .get_pool_data(pool_id)?
            .ok_or_else(|| self.parent.get_pool_data(pool_id))
            .is_ok())
    }

    fn get_delegation_shares(
        &self,
        pool_id: PoolId,
    ) -> Result<Option<BTreeMap<DelegationId, Amount>>, Error> {
        let parent_shares = self.parent.get_pool_delegations_shares(pool_id)?.unwrap_or_default();
        let local_shares = self.get_cached_delegations_shares(pool_id).unwrap_or_default();
        if parent_shares.is_empty() && local_shares.is_empty() {
            Ok(None)
        } else {
            Ok(Some(sum_maps(parent_shares, local_shares)?))
        }
    }

    fn get_delegation_share(
        &self,
        pool_id: PoolId,
        delegation_id: DelegationId,
    ) -> Result<Option<Amount>, Error> {
        let parent_share = self.parent.get_pool_delegation_share(pool_id, delegation_id)?;
        let local_share = self.data.pool_delegation_shares.data().get(&(pool_id, delegation_id));
        combine_amount_delta(&parent_share, &local_share.copied()).map_err(Error::AccountingError)
    }

    fn get_pool_balance(&self, pool_id: PoolId) -> Result<Option<Amount>, Error> {
        let parent_amount = self.parent.get_pool_balance(pool_id)?;
        let local_amount = self.data.pool_balances.data().get(&pool_id);
        combine_amount_delta(&parent_amount, &local_amount.copied()).map_err(Error::AccountingError)
    }

    fn get_delegation_id_balance(
        &self,
        delegation_id: DelegationId,
    ) -> Result<Option<Amount>, Error> {
        let parent_amount = self.parent.get_delegation_balance(delegation_id)?;
        let local_amount = self.data.delegation_balances.data().get(&delegation_id);
        combine_amount_delta(&parent_amount, &local_amount.copied()).map_err(Error::AccountingError)
    }

    fn get_delegation_id_data(&self, id: DelegationId) -> Result<Option<DelegationData>, Error> {
        let parent_data = self.parent.get_delegation_data(id)?;
        let local_data = self.data.delegation_data.data().get(&id);
        combine_data_with_delta(parent_data.as_ref(), local_data).map_err(Error::AccountingError)
    }

    fn get_pool_data(&self, id: PoolId) -> Result<Option<PoolData>, Error> {
        let parent_data = self.parent.get_pool_data(id)?;
        let local_data = self.data.pool_data.data().get(&id);
        combine_data_with_delta(parent_data.as_ref(), local_data).map_err(Error::AccountingError)
    }
}
