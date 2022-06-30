pub mod block;
pub mod config;
pub mod genesis;
mod pow;
pub mod transaction;
mod upgrades;

pub use transaction::*;

pub use config::ChainConfig;
pub use pow::PoWChainConfig;
pub use upgrades::*;

// ================================================== TODO put into a separate module

use crate::primitives::{Idable, Id};
use block::Block;
use genesis::Genesis;

/// Generalized block that's either [genesis::Genesis] or [block::Block].
pub enum GenBlock {
    Genesis(genesis::Genesis),
    Block(block::Block),
}

impl Idable for GenBlock {
    type Tag = GenBlock;
    fn get_id(&self) -> Id<Self::Tag> {
        match self {
            GenBlock::Genesis(g) => g.get_id().into(),
            GenBlock::Block(b) => b.get_id().into(),
        }
    }
}

impl From<Id<Block>> for Id<GenBlock> {
    fn from(id: Id<Block>) -> Id<GenBlock> {
        Id::new(id.get())
    }
}

impl From<Id<Genesis>> for Id<GenBlock> {
    fn from(id: Id<Genesis>) -> Id<GenBlock> {
        Id::new(id.get())
    }
}
