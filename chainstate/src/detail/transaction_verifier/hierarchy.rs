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

use chainstate_types::{storage_result, GenBlockIndex};
use common::{
    chain::{
        tokens::{TokenAuxiliaryData, TokenId},
        Block, GenBlock, OutPoint, OutPointSourceId, Transaction, TxMainChainIndex,
    },
    primitives::{BlockHeight, Id},
};
use utxo::{
    BlockUndo, ConsumedUtxoCache, FlushableUtxoView, Utxo, UtxosStorageRead, UtxosStorageWrite,
    UtxosView,
};

use super::{
    storage::{
        TransactionVerifierStorageError, TransactionVerifierStorageMut,
        TransactionVerifierStorageRef,
    },
    BlockUndoEntry, TransactionVerifier,
};

impl<'a, S: TransactionVerifierStorageRef> TransactionVerifierStorageRef
    for TransactionVerifier<'a, S>
{
    fn get_token_id_from_issuance_tx(
        &self,
        tx_id: Id<Transaction>,
    ) -> Result<Option<TokenId>, TransactionVerifierStorageError> {
        match self.token_issuance_cache.txid_from_issuance().get(&tx_id) {
            Some(v) => return Ok(Some(*v)),
            None => (),
        };
        self.storage_ref.get_token_id_from_issuance_tx(tx_id)
    }

    fn get_gen_block_index(
        &self,
        block_id: &Id<GenBlock>,
    ) -> Result<Option<GenBlockIndex>, storage_result::Error> {
        self.storage_ref.get_gen_block_index(block_id)
    }

    fn get_ancestor(
        &self,
        block_index: &GenBlockIndex,
        target_height: BlockHeight,
    ) -> Result<GenBlockIndex, TransactionVerifierStorageError> {
        self.storage_ref.get_ancestor(block_index, target_height)
    }

    fn get_mainchain_tx_index(
        &self,
        tx_id: &OutPointSourceId,
    ) -> Result<Option<TxMainChainIndex>, TransactionVerifierStorageError> {
        match self.tx_index_cache.get_from_cached(tx_id) {
            Some(v) => match v {
                super::cached_operation::CachedInputsOperation::Write(idx) => {
                    return Ok(Some(idx.clone()))
                }
                super::cached_operation::CachedInputsOperation::Read(idx) => {
                    return Ok(Some(idx.clone()))
                }
                super::cached_operation::CachedInputsOperation::Erase => return Ok(None),
            },
            None => (),
        };
        self.storage_ref.get_mainchain_tx_index(tx_id)
    }

    fn get_token_aux_data(
        &self,
        token_id: &TokenId,
    ) -> Result<Option<TokenAuxiliaryData>, crate::TokensError> {
        match self.token_issuance_cache.data().get(token_id) {
            Some(v) => match v {
                super::token_issuance_cache::CachedTokensOperation::Write(t) => {
                    return Ok(Some(t.clone()))
                }
                super::token_issuance_cache::CachedTokensOperation::Read(t) => {
                    return Ok(Some(t.clone()))
                }
                super::token_issuance_cache::CachedTokensOperation::Erase(_) => return Ok(None),
            },
            None => (),
        }
        self.storage_ref.get_token_aux_data(token_id)
    }
}

impl<'a, S: TransactionVerifierStorageRef> UtxosStorageRead for TransactionVerifier<'a, S> {
    fn get_utxo(&self, outpoint: &OutPoint) -> Result<Option<utxo::Utxo>, storage_result::Error> {
        Ok(self.utxo_cache.utxo(outpoint))
    }

    fn get_best_block_for_utxos(&self) -> Result<Option<Id<GenBlock>>, storage_result::Error> {
        Ok(Some(self.utxo_cache.best_block_hash()))
    }

    fn get_undo_data(
        &self,
        id: Id<Block>,
    ) -> Result<Option<utxo::BlockUndo>, storage_result::Error> {
        match self.utxo_block_undo.get(&id) {
            Some(v) => return Ok(Some(v.undo.clone())),
            None => (),
        };
        self.storage_ref.get_undo_data(id)
    }
}

//TODO: errors?

impl<'a, S: TransactionVerifierStorageMut> TransactionVerifierStorageMut
    for TransactionVerifier<'a, S>
{
    fn set_mainchain_tx_index(
        &mut self,
        tx_id: &OutPointSourceId,
        tx_index: &TxMainChainIndex,
    ) -> Result<(), TransactionVerifierStorageError> {
        self.tx_index_cache.set_tx_index(tx_id, tx_index.clone());
        Ok(())
    }

    fn del_mainchain_tx_index(
        &mut self,
        tx_id: &OutPointSourceId,
    ) -> Result<(), TransactionVerifierStorageError> {
        self.tx_index_cache.del_tx_index(tx_id);
        Ok(())
    }

    fn set_token_aux_data(
        &mut self,
        token_id: &TokenId,
        data: &TokenAuxiliaryData,
    ) -> Result<(), TransactionVerifierStorageError> {
        self.token_issuance_cache.set_token_aux_data(token_id, data.clone());
        Ok(())
    }

    fn del_token_aux_data(
        &mut self,
        token_id: &TokenId,
    ) -> Result<(), TransactionVerifierStorageError> {
        self.token_issuance_cache.del_token_aux_data(token_id);
        Ok(())
    }

    fn set_token_id(
        &mut self,
        issuance_tx_id: &Id<Transaction>,
        token_id: &TokenId,
    ) -> Result<(), TransactionVerifierStorageError> {
        self.token_issuance_cache.set_token_id(issuance_tx_id, token_id);
        Ok(())
    }

    fn del_token_id(
        &mut self,
        issuance_tx_id: &Id<Transaction>,
    ) -> Result<(), TransactionVerifierStorageError> {
        self.token_issuance_cache.del_token_id(issuance_tx_id);
        Ok(())
    }
}

impl<'a, S: TransactionVerifierStorageMut> UtxosStorageWrite for TransactionVerifier<'a, S> {
    fn set_undo_data(
        &mut self,
        id: Id<Block>,
        undo: &BlockUndo,
    ) -> Result<(), storage_result::Error> {
        self.utxo_block_undo.entry(id).or_insert(BlockUndoEntry {
            undo: undo.clone(),
            is_fresh: true,
        });
        Ok(())
    }

    fn del_undo_data(&mut self, id: Id<Block>) -> Result<(), storage_result::Error> {
        self.utxo_block_undo.remove(&id);
        Ok(())
    }

    fn set_utxo(
        &mut self,
        _outpoint: &OutPoint,
        _entry: Utxo,
    ) -> Result<(), storage_result::Error> {
        unreachable!("use FlushableUtxoView::batch_write");
    }

    fn del_utxo(&mut self, _outpoint: &OutPoint) -> Result<(), storage_result::Error> {
        unreachable!("use FlushableUtxoView::batch_write");
    }

    fn set_best_block_for_utxos(
        &mut self,
        _block_id: &Id<GenBlock>,
    ) -> Result<(), storage_result::Error> {
        unreachable!("use FlushableUtxoView::batch_write");
    }
}

impl<'a, S: TransactionVerifierStorageRef> FlushableUtxoView for TransactionVerifier<'a, S> {
    fn batch_write(&mut self, utxos: ConsumedUtxoCache) -> Result<(), utxo::Error> {
        self.utxo_cache.batch_write(utxos)
    }
}
