use jsonrpc_macros::build_rpc_trait;
use jsonrpc_macros::jsonrpc_core::Error;

use jsonrpc_macros::Trailing;

use crate::modules::types::GetBlockResponse;
use crate::modules::types::GetTxOutResponse;
use crate::modules::types::GetTxOutSetInfoResponse;
use crate::modules::types::H256;

build_rpc_trait! {
    ///  blockchain data interface.
    pub trait BlockChain {
        /// Get hash of best block.
        /// @curl-example: curl --data-binary '{"jsonrpc": "2.0", "method": "getbestblockhash", "params": [], "id":1 }' -H 'content-type: application/json' http://127.0.0.1:8332/
        #[rpc(name = "getbestblockhash")]
        fn best_block_hash(&self) -> Result<H256, Error>;
        /// Get height of best block.
        /// @curl-example: curl --data-binary '{"jsonrpc": "2.0", "method": "getblockcount", "params": [], "id":1 }' -H 'content-type: application/json' http://127.0.0.1:8332/
        #[rpc(name = "getblockcount")]
        fn block_count(&self) -> Result<u32, Error>;
        /// Get hash of block at given height.
        /// @curl-example: curl --data-binary '{"jsonrpc": "2.0", "method": "getblockhash", "params": [0], "id":1 }' -H 'content-type: application/json' http://127.0.0.1:8332/
        #[rpc(name = "getblockhash")]
        fn block_hash(&self, u32) -> Result<H256, Error>;
        /// Get proof-of-work difficulty as a multiple of the minimum difficulty
        /// @curl-example: curl --data-binary '{"jsonrpc": "2.0", "method": "getdifficulty", "params": [], "id":1 }' -H 'content-type: application/json' http://127.0.0.1:8332/
        #[rpc(name = "getdifficulty")]
        fn difficulty(&self) -> Result<f64, Error>;
        /// Get information on given block.
        /// @curl-example: curl --data-binary '{"jsonrpc": "2.0", "method": "getblock", "params": ["000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f"], "id":1 }' -H 'content-type: application/json' http://127.0.0.1:8332/
        #[rpc(name = "getblock")]
        fn block(&self, H256, Trailing<bool>) -> Result<GetBlockResponse, Error>;
        /// Get details about an unspent transaction output.
        /// @curl-example: curl --data-binary '{"jsonrpc": "2.0", "method": "gettxout", "params": ["4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b", 0], "id":1 }' -H 'content-type: application/json' http://127.0.0.1:8332/
        #[rpc(name = "gettxout")]
        fn transaction_out(&self, H256, u32, Trailing<bool>) -> Result<GetTxOutResponse, Error>;
        /// Get statistics about the unspent transaction output set.
        /// @curl-example: curl --data-binary '{"jsonrpc": "2.0", "method": "gettxoutsetinfo", "params": [], "id":1 }' -H 'content-type: application/json' http://127.0.0.1:8332/
        #[rpc(name = "gettxoutsetinfo")]
        fn transaction_out_set_info(&self) -> Result<GetTxOutSetInfoResponse, Error>;
    }
}
