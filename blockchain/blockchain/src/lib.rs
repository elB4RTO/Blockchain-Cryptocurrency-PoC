pub mod block;
pub mod blockchain;


use bytes_utils::BytesConversion;


static BLOCKCHAIN: std::sync::Mutex<blockchain::Blockchain> = std::sync::Mutex::new(blockchain::Blockchain::invalid());


pub fn init() {
	*BLOCKCHAIN.lock().unwrap() = blockchain::Blockchain::new();
}

pub fn as_actix_bytes() -> Result<actix_web::web::Bytes, String> {
	BLOCKCHAIN.lock().unwrap().as_actix_bytes()
}

pub fn update_from_bytes(bytes: actix_web::web::Bytes) -> Result<(), String> {
	let bc = blockchain::Blockchain::from_actix_bytes(bytes)?;
	BLOCKCHAIN.lock().unwrap().update(bc)
		.map_err(|e| format!("blockchain::update_from_bytes(): failed to validate: {:?}", e))
}

pub fn update(bc: blockchain::Blockchain) -> Result<(), String> {
	BLOCKCHAIN.lock().unwrap().update(bc)
		.map_err(|e| format!("blockchain::update(): failed to validate: {:?}", e))
}

pub fn validate_blockchain(bc: &blockchain::Blockchain) -> blockchain::ValidationResult {
	blockchain::validate_blockchain(&BLOCKCHAIN.lock().unwrap(), bc)
}

pub fn validate_last_block(b: &block::Block) -> block::ValidationResult {
	block::validate_block(&BLOCKCHAIN.lock().unwrap().last_block(), b)
}

pub fn validate_new_block(b: &block::Block) -> block::ValidationResult {
	block::validate_new_block(&BLOCKCHAIN.lock().unwrap().last_block(), b)
}

pub fn add_block(b: block::Block) {
	BLOCKCHAIN.lock().unwrap().add_block(b);
}

pub fn mine_block(data_ref: &[u8]) -> Result<block::Block, String> {
	let data : block::BlockData = data_ref.try_into()
		.map_err(|_| String::from("blockchain::mine_block(): invalid block data"))?;
	let mut bc = BLOCKCHAIN.lock().unwrap();
	bc.mine_block(data);
	Ok(bc.last_block().clone())
}


#[cfg(test)]
pub(crate) mod test_utils {
	use super::*;
	
    pub(crate) fn random_timestamp() -> i64 {
		use rand::Rng;
		rand::rng().random::<i64>()
	}
    
    pub(crate) fn random_bytes<const N: usize>() -> [u8; N] {
		use rand::Fill;
		let ref mut rng = rand::rng();
		let mut bytes : [u8; N] = [0; N];
		bytes.fill(rng);
		bytes
	}
	
	pub(crate) fn random_block(last_block: &block::Block) -> block::Block {
		block::mine_block(last_block, random_bytes::<{block::BLOCK_DATA_LEN}>())
	}
	
	pub(crate) fn fully_random_block() -> block::Block {
		block::Block::new(random_timestamp(), random_bytes::<{block::BLOCK_HASH_LEN}>(), random_bytes::<{block::BLOCK_DATA_LEN}>())
	}
}
