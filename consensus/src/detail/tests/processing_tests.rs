// Copyright (c) 2022 RBB S.r.l
// opensource@mintlayer.org
// SPDX-License-Identifier: MIT
// Licensed under the MIT License;
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://spdx.org/licenses/MIT
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// Author(s): S. Afach, A. Sinitsyn

use crate::detail::tests::test_framework::BlockTestFrameWork;
use crate::detail::tests::*;
use blockchain_storage::Store;
use common::chain::block::consensus_data::PoWData;
use common::chain::config::ChainConfigBuilder;
use common::chain::OutputSpentState;
use common::primitives::Compact;

#[test]
fn test_process_genesis_block_wrong_block_source() {
    common::concurrency::model(|| {
        // Genesis can't be from Peer, test it
        let config = create_unit_test_config();
        let storage = Store::new_empty().unwrap();
        let mut consensus = Consensus::new_no_genesis(config.clone(), storage).unwrap();

        // process the genesis block
        let block_source = BlockSource::Peer(0);
        let result = consensus.process_block(config.genesis_block().clone(), block_source);
        assert_eq!(result, Err(BlockError::InvalidBlockSource));
    });
}

#[test]
fn test_process_genesis_block() {
    common::concurrency::model(|| {
        // This test process only Genesis block
        let config = create_unit_test_config();
        let storage = Store::new_empty().unwrap();
        let mut consensus = Consensus::new_no_genesis(config, storage).unwrap();

        // process the genesis block
        let block_source = BlockSource::Local;
        let block_index = consensus
            .process_block(consensus.chain_config.genesis_block().clone(), block_source)
            .ok()
            .flatten()
            .unwrap();
        assert_eq!(
            consensus
                .blockchain_storage
                .get_best_block_id()
                .expect(ERR_BEST_BLOCK_NOT_FOUND),
            Some(consensus.chain_config.genesis_block().get_id())
        );
        assert_eq!(block_index.get_prev_block_id(), &None);
        assert_eq!(block_index.get_chain_trust(), 1);
        assert_eq!(block_index.get_block_height(), BlockHeight::new(0));
    });
}

#[test]
fn test_orphans_chains() {
    common::concurrency::model(|| {
        let config = create_unit_test_config();
        let storage = Store::new_empty().unwrap();
        let mut consensus = Consensus::new(config, storage).unwrap();

        // Process the orphan block
        let new_block = consensus.chain_config.genesis_block().clone();
        for _ in 0..255 {
            let new_block = produce_test_block(&consensus.chain_config, &new_block, true);
            assert_eq!(
                consensus.process_block(new_block.clone(), BlockSource::Local),
                Err(BlockError::Orphan)
            );
        }
    });
}

#[test]
fn test_empty_consensus() {
    common::concurrency::model(|| {
        // No genesis
        let config = create_unit_test_config();
        let storage = Store::new_empty().unwrap();
        let consensus = Consensus::new_no_genesis(config, storage).unwrap();
        assert!(consensus.get_best_block_id().unwrap().is_none());
        assert!(consensus
            .blockchain_storage
            .get_block(consensus.chain_config.genesis_block().get_id())
            .unwrap()
            .is_none());
        // Let's add genesis
        let config = create_unit_test_config();
        let storage = Store::new_empty().unwrap();
        let consensus = Consensus::new(config, storage).unwrap();
        assert!(consensus.get_best_block_id().unwrap().is_some());
        assert!(
            consensus.get_best_block_id().ok().flatten().unwrap()
                == consensus.chain_config.genesis_block().get_id()
        );
        assert!(consensus
            .blockchain_storage
            .get_block(consensus.chain_config.genesis_block().get_id())
            .unwrap()
            .is_some());
        assert!(
            consensus
                .blockchain_storage
                .get_block(consensus.chain_config.genesis_block().get_id())
                .unwrap()
                .unwrap()
                .get_id()
                == consensus.chain_config.genesis_block().get_id()
        );
    });
}

#[test]
fn test_spend_inputs_simple() {
    common::concurrency::model(|| {
        let mut consensus = setup_consensus();

        // Create a new block
        let block = produce_test_block(
            &consensus.chain_config,
            consensus.chain_config.genesis_block(),
            false,
        );

        // Check that all tx not in the main chain
        for tx in block.transactions() {
            assert!(
                consensus
                    .blockchain_storage
                    .get_mainchain_tx_index(&OutPointSourceId::from(tx.get_id()))
                    .expect(ERR_STORAGE_FAIL)
                    == None
            );
        }

        // Process the second block
        let new_id = Some(block.get_id());
        assert!(consensus.process_block(block.clone(), BlockSource::Local).is_ok());
        assert_eq!(
            consensus
                .blockchain_storage
                .get_best_block_id()
                .expect(ERR_BEST_BLOCK_NOT_FOUND),
            new_id
        );

        // Check that tx inputs in the main chain and not spend
        for tx in block.transactions() {
            let tx_index = consensus
                .blockchain_storage
                .get_mainchain_tx_index(&OutPointSourceId::from(tx.get_id()))
                .expect("Not found mainchain tx index")
                .expect(ERR_STORAGE_FAIL);

            for input in tx.get_inputs() {
                if tx_index
                    .get_spent_state(input.get_outpoint().get_output_index())
                    .expect("Unable to get spent state")
                    != OutputSpentState::Unspent
                {
                    panic!("Tx input can't be spent");
                }
            }
        }
    });
}

#[test]
fn test_straight_chain() {
    common::concurrency::model(|| {
        const COUNT_BLOCKS: usize = 255;
        // In this test, processing a few correct blocks in a single chain
        let config = create_unit_test_config();
        let storage = Store::new_empty().unwrap();
        let mut consensus = Consensus::new_no_genesis(config, storage).unwrap();

        // process the genesis block
        let block_source = BlockSource::Local;
        let mut block_index = consensus
            .process_block(consensus.chain_config.genesis_block().clone(), block_source)
            .ok()
            .flatten()
            .expect("Unable to process genesis block");
        assert_eq!(
            consensus
                .blockchain_storage
                .get_best_block_id()
                .expect(ERR_BEST_BLOCK_NOT_FOUND),
            Some(consensus.chain_config.genesis_block().get_id())
        );
        assert_eq!(
            block_index.get_block_id(),
            &consensus.chain_config.genesis_block().get_id()
        );
        assert_eq!(block_index.get_prev_block_id(), &None);
        // TODO: ensure that block at height is tested after removing the next
        assert_eq!(block_index.get_chain_trust(), 1);
        assert_eq!(block_index.get_block_height(), BlockHeight::new(0));

        let mut prev_block = consensus.chain_config.genesis_block().clone();
        for _ in 0..COUNT_BLOCKS {
            let prev_block_id = block_index.get_block_id();
            let best_block_id = consensus
                .blockchain_storage
                .get_best_block_id()
                .ok()
                .flatten()
                .expect("Unable to get best block ID");
            assert_eq!(&best_block_id, block_index.get_block_id());
            let block_source = BlockSource::Peer(1);
            let new_block = produce_test_block(&consensus.chain_config, &prev_block, false);
            let new_block_index = dbg!(consensus.process_block(new_block.clone(), block_source))
                .ok()
                .flatten()
                .expect("Unable to process block");

            // TODO: ensure that block at height is tested after removing the next
            assert_eq!(
                new_block_index.get_prev_block_id().as_ref(),
                Some(prev_block_id)
            );
            assert!(new_block_index.get_chain_trust() > block_index.get_chain_trust());
            assert_eq!(
                new_block_index.get_block_height(),
                block_index.get_block_height().next_height()
            );

            block_index = new_block_index;
            prev_block = new_block;
        }
    });
}

#[test]
fn test_get_ancestor() {
    use crate::detail::tests::test_framework::BlockTestFrameWork;
    let mut btf = BlockTestFrameWork::new();
    btf.create_chain(&btf.genesis().get_id(), 3).expect("Chain creation to succeed");
    let block_2_index = btf.block_indexes[2].clone();
    let block_1_index = btf.block_indexes[1].clone();
    let block_0_index = btf.block_indexes[0].clone();

    assert_eq!(
        block_1_index,
        btf.consensus
            .make_db_tx()
            .get_ancestor(&block_2_index, 1.into())
            .expect("ancestor")
    );

    assert_eq!(
        block_0_index,
        btf.consensus
            .make_db_tx()
            .get_ancestor(&block_2_index, 0.into())
            .expect("ancestor")
    );

    assert_eq!(
        block_0_index,
        btf.consensus
            .make_db_tx()
            .get_ancestor(&block_0_index, 0.into())
            .expect("ancestor")
    );
    // ERROR
    assert_eq!(
        Err(BlockError::InvalidAncestorHeight {
            ancestor_height: 2.into(),
            block_height: 1.into(),
        }),
        btf.consensus.make_db_tx().get_ancestor(&block_1_index, 2.into())
    );
}

/*
struct DummyBlockIndexHandle {
    block_indices: HashMap<H256, BlockIndex>,
}

impl DummyBlockIndexHandle {
    fn new(block_indices: Vec<BlockIndex>) -> Self {
        Self {
            block_indices: block_indices
                .into_iter()
                .map(|block_index| (block_index.get_block_id().get(), block_index))
                .collect(),
        }
    }
}

impl BlockIndexHandle for DummyBlockIndexHandle {
    fn get_block_index(
        &self,
        block_id: &Id<Block>,
    ) -> blockchain_storage::Result<Option<BlockIndex>> {
        Ok(self.block_indices.get(&block_id.get()).cloned())
    }
    fn get_ancestor(
        &self,
        _block_index: &BlockIndex,
        ancestor_height: BlockHeight,
    ) -> Result<BlockIndex, BlockError> {
        self.block_indices
            .iter()
            .find_map(|(_id, block_index)| {
                if block_index.get_block_height() == ancestor_height {
                    Some(block_index)
                } else {
                    None
                }
            })
            .cloned()
            .ok_or(BlockError::NotFound)
    }
}

*/

#[test]
fn test_consensus_type() {
    use common::chain::ConsensusUpgrade;
    use common::chain::NetUpgrades;
    use common::chain::UpgradeVersion;
    use common::Uint256;

    let ignore_consensus = BlockHeight::new(0);
    let pow = BlockHeight::new(5);
    let ignore_again = BlockHeight::new(10);
    let pow_again = BlockHeight::new(15);

    let min_difficulty =
        Uint256([0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF]);

    let upgrades = vec![
        (
            ignore_consensus,
            UpgradeVersion::ConsensusUpgrade(ConsensusUpgrade::IgnoreConsensus),
        ),
        (
            pow,
            UpgradeVersion::ConsensusUpgrade(ConsensusUpgrade::PoW {
                initial_difficulty: min_difficulty.into(),
            }),
        ),
        (
            ignore_again,
            UpgradeVersion::ConsensusUpgrade(ConsensusUpgrade::IgnoreConsensus),
        ),
        (
            pow_again,
            UpgradeVersion::ConsensusUpgrade(ConsensusUpgrade::PoW {
                initial_difficulty: min_difficulty.into(),
            }),
        ),
    ];

    let net_upgrades = NetUpgrades::initialize(upgrades);

    // Internally this calls Consensus::new, which processes the genesis block
    // This should succeed because ChainConfigBuilder by default uses create_mainnet_genesis to
    // create the genesis_block, and this function creates a genesis block with
    // ConsenssuData::None, which agreess with the net_upgrades we defined above.
    let config = ChainConfigBuilder::new().with_net_upgrades(net_upgrades).build();
    let consensus = ConsensusBuilder::new().with_config(config.clone()).build();

    let mut btf = BlockTestFrameWork::with_consensus(consensus);
    println!("btf len after creation:{}", btf.block_indexes.len());

    // The next block will have height 1. At this height, we are still under IngoreConsenssu, so
    // processing a block with PoWData will fail
    let pow_block = produce_test_block_with_consensus_data(
        &config,
        btf.genesis(),
        false,
        ConsensusData::PoW(PoWData::new(Compact(0), 0, vec![])),
    );
    assert!(matches!(
        btf.add_special_block(pow_block),
        Err(BlockError::ConsensusTypeMismatch(..))
    ));

    // Create 4 more blocks with Consensus Nonw
    btf.create_chain(&btf.genesis().get_id(), 4).expect("chain creation");

    // The next block will be at height 5, so it is expected to be a PoW block. Let's crate a block
    // with ConsensusData::None and see that adding it fails
    let block_without_consensus_data = produce_test_block_with_consensus_data(
        &config,
        &btf.get_block(btf.block_indexes[4].get_block_id().clone()).unwrap().unwrap(),
        false,
        ConsensusData::None,
    );
    assert!(matches!(
        btf.add_special_block(block_without_consensus_data),
        Err(BlockError::ConsensusTypeMismatch(..))
    ));
    println!(
        "btf blocks len before mining loop:{}",
        btf.block_indexes.len()
    );

    // Mine blocks 5-9 with minimal difficulty, as expected by net upgrades
    for i in 5..10 {
        println!("i={}", i);
        let prev_block =
            btf.get_block(btf.block_indexes[i - 1].get_block_id().clone()).unwrap().unwrap();
        let mut mined_block = btf.random_block(&prev_block, None);
        let bits = min_difficulty.into();
        assert!(
            crate::detail::pow::work::mine(&mut mined_block, u128::MAX, bits, vec![])
                .expect("Unexpected conversion error")
        );
        assert!(btf.add_special_block(mined_block).is_ok());
    }
    println!(
        "btf blocks len after mining loop:{}",
        btf.block_indexes.len()
    );

    // Block 10 should ignore consensus according to net upgrades. The following Pow block should
    // fail.
    let prev_block = btf.get_block(btf.block_indexes[9].get_block_id().clone()).unwrap().unwrap();
    let mut mined_block = btf.random_block(&prev_block, None);
    let bits = min_difficulty.into();
    assert!(
        crate::detail::pow::work::mine(&mut mined_block, u128::MAX, bits, vec![])
            .expect("Unexpected conversion error")
    );
    assert!(matches!(
        btf.add_special_block(mined_block),
        Err(BlockError::ConsensusTypeMismatch(..))
    ));

    // Create blocks 10-14 without consensus data as required by net_upgrades
    btf.create_chain(&prev_block.get_id(), 5).expect("chain creation");

    // At height 15 we are again proof of work, ignoring consensus should fail
    let prev_block = btf.get_block(btf.block_indexes[14].get_block_id().clone()).unwrap().unwrap();
    let block_without_consensus_data =
        produce_test_block_with_consensus_data(&config, &prev_block, false, ConsensusData::None);
    assert!(matches!(
        btf.add_special_block(block_without_consensus_data),
        Err(BlockError::ConsensusTypeMismatch(..))
    ));

    // Mining should work
    for i in 15..20 {
        println!("i={}", i);
        let prev_block =
            btf.get_block(btf.block_indexes[i - 1].get_block_id().clone()).unwrap().unwrap();
        let mut mined_block = btf.random_block(&prev_block, None);
        let bits = min_difficulty.into();
        assert!(
            crate::detail::pow::work::mine(&mut mined_block, u128::MAX, bits, vec![])
                .expect("Unexpected conversion error")
        );
        assert!(btf.add_special_block(mined_block).is_ok());
    }
}