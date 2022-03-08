use crate::block;
use leveldb::database::Database;
use storage::common::BlockKey;
use block::Block;
use bigint::U256;
use storage::block_db::BlockChainDb;
use utils::coder;

pub struct BlockChain {
    pub blocks: Vec<block::Block>,
    blocks_db: Box<Database<BlockKey>>
}


impl BlockChain {

    fn write_block_to_db(database: &mut Database<BlockKey>, block: &Block) {
        let key = BlockKey{val: U256::from(block.hash)};
        let value = coder::block_serialize(&block);
        BlockChainDb::write_db(database, key, &value);
    }

    fn write_tail_to_db(mut database: &mut Database<BlockKey>, block: &Block) {
        let key = BlockKey{val: U256::from("tail".as_bytes())};
        let value = coder::block_serialize(&(block.hash));
        BlockChainDb::write_db(&mut database, key, &value);
    }
    
    pub fn add_block(&mut self, transaction: String) {
        println!("++++++++ 当前链长度： ==== {}", self.blocks.len());
        let pre_block = &self.blocks[self.blocks.len() - 1];
        let new_block = Block::new(pre_block.hash.clone(), transaction);
        BlockChain::write_block_to_db(&mut (self.blocks_db), &new_block);
        println!("已成功将区块写入数据库");
        self.blocks.push(new_block); 

    }

    fn new_genesis_block() -> block::Block {
        Block::new([0; 32],"创世区块：".to_string())
    }

    
    pub fn new_blockchain() -> BlockChain {

        //create a database
        let mut database = BlockChainDb::new("blockchain_db");

        let genesis = BlockChain::new_genesis_block();

        //write genesis to database
        BlockChain::write_block_to_db(&mut database, &genesis);

        

        BlockChain {
            blocks: vec![genesis],
            blocks_db : Box::new(database)
        }
    }
}