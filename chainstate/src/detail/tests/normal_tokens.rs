use std::vec;

use crate::{
    detail::{CheckBlockError, CheckBlockTransactionsError},
    BlockError, BlockSource, Chainstate,
};

use super::{anyonecanspend_address, setup_chainstate};
use chainstate_types::block_index::BlockIndex;
use common::{
    chain::{
        block::{timestamp::BlockTimestamp, Block, ConsensusData},
        signature::inputsig::InputWitness,
        AssetData, OutputPurpose, OutputValue, Transaction, TxInput, TxOutput,
    },
    primitives::{Amount, Idable},
};

fn process_token(
    chainstate: &mut Chainstate,
    value: OutputValue,
) -> Result<Option<BlockIndex>, BlockError> {
    let prev_block_id = chainstate.get_best_block_id().unwrap().unwrap();
    let receiver = anyonecanspend_address();

    let prev_block = chainstate.get_block(prev_block_id.clone()).unwrap().unwrap();
    // Create a token issue transaction and block
    let inputs = vec![TxInput::new(
        prev_block.transactions()[0].get_id().into(),
        0,
        InputWitness::NoSignature(None),
    )];
    let outputs = vec![TxOutput::new(value, OutputPurpose::Transfer(receiver.clone()))];
    let block = Block::new(
        vec![Transaction::new(0, inputs, outputs, 0).unwrap()],
        Some(prev_block_id),
        BlockTimestamp::from_int_seconds(prev_block.timestamp().as_int_seconds() + 1),
        ConsensusData::None,
    )
    .unwrap();

    // Process it
    chainstate.process_block(block, BlockSource::Local)
}

#[test]
fn token_issue_test() {
    common::concurrency::model(|| {
        // Process genesis
        let mut chainstate = setup_chainstate();
        let value = OutputValue::Asset(AssetData::TokenIssuanceV1 {
            token_ticker: b"USDC".to_vec(),
            amount_to_issue: Amount::from_atoms(52292852472),
            number_of_decimals: 1,
            metadata_uri: b"https://some_site.meta".to_vec(),
        });
        let block_index = process_token(&mut chainstate, value.clone()).unwrap().unwrap();
        let block = chainstate.get_block(block_index.block_id().clone()).unwrap().unwrap();
        assert_eq!(block.transactions()[0].outputs()[0].value(), &value);

        // Try to create TX with incorrect token issue data
        // Name is too long
        let value = OutputValue::Asset(AssetData::TokenIssuanceV1 {
            token_ticker: b"TRY TO USE THE LONG NAME".to_vec(),
            amount_to_issue: Amount::from_atoms(52292852472),
            number_of_decimals: 1,
            metadata_uri: b"https://some_site.meta".to_vec(),
        });
        let block_index = dbg!(process_token(&mut chainstate, value.clone()));
        assert!(matches!(
            block_index,
            Err(BlockError::CheckBlockFailed(
                CheckBlockError::CheckTransactionFailed(
                    CheckBlockTransactionsError::TokenIssueTransactionIncorrect(_, _)
                )
            ))
        ));

        // let block = chainstate.get_block(block_index.block_id().clone()).unwrap().unwrap();
        // assert_eq!(block.transactions()[0].outputs()[0].value(), &value);
        // let outputs = vec![TxOutput::new(value, OutputPurpose::Transfer(receiver.clone()))];
        // Doesn't exist name
        // Name contain not alpha-numeric byte
        // Issue amount is too big
        // Issue amount is too low
        // Too many decimals
        // URI is too long
    });
}
