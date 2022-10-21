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

use chainstate::{calculate_median_time_past, BlockError, TransactionVerificationStrategy};
use chainstate_types::{BlockIndex, BlockIndexHandle};
use common::{
    chain::{Block, ChainConfig},
    primitives::{id::WithId, Amount, Idable},
};
use tx_verifier::transaction_verifier::{
    error::ConnectTransactionError, flush::flush_to_storage,
    storage::TransactionVerifierStorageRef, BlockTransactableRef, Fee, Subsidy,
    TransactionVerifier,
};
use utils::tap_error_log::LogError;

/// Strategy that creates separate instances of TransactionVerifier on every tx, flushing the
/// result to a single TransactionVerifier that is returned from the connect/disconnect functions.
/// For now this is only used for testing purposes.
pub struct DisposableTransactionVerificationStrategy {}

impl DisposableTransactionVerificationStrategy {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for DisposableTransactionVerificationStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl TransactionVerificationStrategy for DisposableTransactionVerificationStrategy {
    fn connect_block<'a, H, S, M>(
        &self,
        tx_verifier_maker: M,
        block_index_handle: &'a H,
        storage_backend: &'a S,
        chain_config: &'a ChainConfig,
        block_index: &'a BlockIndex,
        block: &WithId<Block>,
    ) -> Result<TransactionVerifier<'a, S>, BlockError>
    where
        H: BlockIndexHandle,
        S: TransactionVerifierStorageRef,
        M: Fn(&'a S, &'a ChainConfig) -> TransactionVerifier<'a, S>,
    {
        // The comparison for timelock is done with median_time_past based on BIP-113, i.e., the median time instead of the block timestamp
        let median_time_past =
            calculate_median_time_past(block_index_handle, &block.prev_block_id());

        let mut base_tx_verifier = tx_verifier_maker(storage_backend, chain_config);

        let reward_fees = base_tx_verifier
            .connect_transactable(
                block_index,
                BlockTransactableRef::BlockReward(block),
                &block_index.block_height(),
                &median_time_past,
            )
            .log_err()?;
        debug_assert!(reward_fees.is_none());

        let total_fees = block
            .transactions()
            .iter()
            .enumerate()
            .try_fold(Amount::from_atoms(0), |total, (tx_num, _)| {
                let mut tx_verifier = base_tx_verifier.derive_child();
                let fee = tx_verifier
                    .connect_transactable(
                        block_index,
                        BlockTransactableRef::Transaction(block, tx_num),
                        &block_index.block_height(),
                        &median_time_past,
                    )
                    .map_err(BlockError::StateUpdateFailed)
                    .log_err()?;
                let consumed_cache = tx_verifier.consume()?;
                flush_to_storage(&mut base_tx_verifier, consumed_cache)
                    .map_err(BlockError::TransactionVerifierError)
                    .log_err()?;

                (total + fee.expect("connect tx should return fees").0).ok_or_else(|| {
                    BlockError::StateUpdateFailed(
                        ConnectTransactionError::FailedToAddAllFeesOfBlock(block.get_id()),
                    )
                })
            })
            .log_err()?;

        let block_subsidy = chain_config.block_subsidy_at_height(&block_index.block_height());
        base_tx_verifier
            .check_block_reward(block, Fee(total_fees), Subsidy(block_subsidy))
            .log_err()?;

        base_tx_verifier.set_best_block(block.get_id().into());

        Ok(base_tx_verifier)
    }

    fn disconnect_block<'a, S, M>(
        &self,
        tx_verifier_maker: M,
        storage_backend: &'a S,
        chain_config: &'a ChainConfig,
        block: &WithId<Block>,
    ) -> Result<TransactionVerifier<'a, S>, BlockError>
    where
        S: TransactionVerifierStorageRef,
        M: Fn(&'a S, &'a ChainConfig) -> TransactionVerifier<'a, S>,
    {
        let mut base_tx_verifier = tx_verifier_maker(storage_backend, chain_config);

        block
            .transactions()
            .iter()
            .enumerate()
            .rev()
            .try_for_each(|(tx_num, _)| {
                let mut tx_verifier = base_tx_verifier.derive_child();

                tx_verifier
                    .disconnect_transactable(BlockTransactableRef::Transaction(block, tx_num))
                    .log_err()?;

                let consumed_cache = tx_verifier.consume()?;
                flush_to_storage(&mut base_tx_verifier, consumed_cache)
                    .map_err(BlockError::TransactionVerifierError)
            })
            .log_err()?;

        let mut tx_verifier = base_tx_verifier.derive_child();
        tx_verifier
            .disconnect_transactable(BlockTransactableRef::BlockReward(block))
            .log_err()?;
        let consumed_cache = tx_verifier.consume()?;
        flush_to_storage(&mut base_tx_verifier, consumed_cache)?;

        base_tx_verifier.set_best_block(block.prev_block_id());

        Ok(base_tx_verifier)
    }
}