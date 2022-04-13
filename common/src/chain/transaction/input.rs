use crate::chain::{block::Block, transaction::Transaction};
use crate::primitives::{Id, H256};
use parity_scale_codec::{Decode, Encode};

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub enum OutPointSourceId {
    #[codec(index = 0)]
    Transaction(Id<Transaction>),
    #[codec(index = 1)]
    BlockReward(Id<Block>),
}

impl From<Id<Transaction>> for OutPointSourceId {
    fn from(id: Id<Transaction>) -> OutPointSourceId {
        OutPointSourceId::Transaction(id)
    }
}

impl From<Id<Block>> for OutPointSourceId {
    fn from(id: Id<Block>) -> OutPointSourceId {
        OutPointSourceId::BlockReward(id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct OutPoint {
    id: OutPointSourceId,
    index: u32,
}

fn outpoint_source_id_as_monolithic_tuple(v: &OutPointSourceId) -> (u8, H256) {
    let tx_out_index = 0;
    let blk_reward_index = 1;
    match v {
        OutPointSourceId::Transaction(h) => (tx_out_index, h.get()),
        OutPointSourceId::BlockReward(h) => (blk_reward_index, h.get()),
    }
}

impl PartialOrd for OutPoint {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let id = outpoint_source_id_as_monolithic_tuple(&self.id);
        let other_id = outpoint_source_id_as_monolithic_tuple(&other.id);

        (id, self.index).partial_cmp(&(other_id, other.index))
    }
}

impl Ord for OutPoint {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let id = outpoint_source_id_as_monolithic_tuple(&self.id);
        let other_id = outpoint_source_id_as_monolithic_tuple(&other.id);

        (id, self.index).cmp(&(other_id, other.index))
    }
}

impl OutPoint {
    pub fn new(outpoint_source_id: OutPointSourceId, output_index: u32) -> Self {
        OutPoint {
            id: outpoint_source_id,
            index: output_index,
        }
    }

    pub fn get_tx_id(&self) -> OutPointSourceId {
        self.id.clone()
    }

    pub fn get_output_index(&self) -> u32 {
        self.index
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct TxInput {
    outpoint: OutPoint,
    witness: Vec<u8>,
}

impl TxInput {
    pub fn new(outpoint_source_id: OutPointSourceId, output_index: u32, witness: Vec<u8>) -> Self {
        TxInput {
            outpoint: OutPoint::new(outpoint_source_id, output_index),
            witness,
        }
    }

    pub fn get_outpoint(&self) -> &OutPoint {
        &self.outpoint
    }

    pub fn get_witness(&self) -> &Vec<u8> {
        &self.witness
    }
}
