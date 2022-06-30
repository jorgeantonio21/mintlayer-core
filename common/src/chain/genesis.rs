use super::TxOutput;
use super::block::timestamp::BlockTimestamp;
use crate::primitives::{Idable, Id, id};

use serialization::{Encode, Decode};

/// Genesis defines the initial state of the blockchain
#[derive(Clone, Encode, Decode)]
pub struct Genesis {
    /// Magic bytes to identify the chain
    magic_bytes: [u8; 4],
    /// The initial UTXO set
    utxos: Vec<TxOutput>,
    /// Timestamp (is this needed?)
    timestamp: BlockTimestamp,
}

impl Genesis {
    pub fn new(magic_bytes: [u8; 4], utxos: Vec<TxOutput>, timestamp: BlockTimestamp) -> Self {
        Self {
            magic_bytes,
            utxos,
            timestamp,
        }
    }

    pub fn magic_bytes(&self) -> [u8; 4] {
        self.magic_bytes
    }

    pub fn utxos(&self) -> &[TxOutput] {
        &self.utxos
    }

    pub fn timestamp(&self) -> BlockTimestamp {
        self.timestamp
    }
}

impl Idable for Genesis {
    type Tag = Genesis;
    fn get_id(&self) -> Id<Self::Tag> {
        Id::new(&id::hash_encoded(&self))
    }
}
