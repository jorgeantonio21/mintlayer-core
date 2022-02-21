use crate::chain::{block::Block, transaction::Transaction};
use crate::primitives::Id;
use parity_scale_codec::{Decode, Encode};

#[derive(Debug, Ord, PartialOrd, Clone, PartialEq, Eq, Encode, Decode)]
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

impl OutPointSourceId {
    pub fn get_tx_id(&self) -> Option<&Id<Transaction>> {
        match self {
            OutPointSourceId::Transaction(id) => Some(id),
            _ => None,
        }
    }
}

#[derive(Debug, PartialOrd, Ord, Clone, PartialEq, Eq, Encode, Decode)]
pub struct OutPoint {
    id: OutPointSourceId,
    index: u32,
}

impl OutPoint {
    pub const COINBASE_OUTPOINT_INDEX: u32 = u32::MAX;

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

    pub fn is_coinbase(&self) -> bool {
        matches!(self.id, OutPointSourceId::BlockReward(..))
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, PartialEq, Eq, Encode, Decode)]
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
