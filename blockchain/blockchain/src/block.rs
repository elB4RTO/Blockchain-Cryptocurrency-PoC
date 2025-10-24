use sha2::Digest;


/// The length on a hash block in Bytes
pub const BLOCK_HASH_LEN : usize = 64;
/// The length on a data block in Bytes
pub const BLOCK_DATA_LEN : usize = 1024;


pub type Timestamp = i64;
/// The type of a hash block
pub type BlockHash = [u8; BLOCK_HASH_LEN];
/// The type of a data block
pub type BlockData = [u8; BLOCK_DATA_LEN];


/// Result type returned on [`Blockchain`] validation
pub type ValidationResult = Result<ValidationResponse, ValidationError>;

/// The [`Ok`] type of a [`ValidationResult`]
pub enum ValidationResponse {
    ValidBlock,
}

/// The [`Err`] type of a [`ValidationResult`]
#[derive(Debug)]
pub enum ValidationError {
    DifferentTimestamp,
    DifferentData,
    DifferentHash,
    DifferentPreviousHash,
    EarlierTimestamp,
    TamperedHash,
}


/// Represents a block in the blockchain
#[derive(PartialEq, Clone, Debug)]
#[repr(C)]
pub struct Block {
    /// Timestamp in UNIX epoch of the creation of the block
    timestamp: Timestamp,
    /// The hash of the previous block in the chain
    prev_hash: BlockHash,
    /// The hash of the block
    this_hash: BlockHash,
    /// The data stored in the block
    data: BlockData,
}

impl Block {
    /// Creates a new genesis [`Block`]
    pub fn genesis() -> Self {
        Self::new(0, [0; BLOCK_HASH_LEN], [0; BLOCK_DATA_LEN])
    }

    /// Creates a new [`Block`] with the given data
    ///
    /// The hash of the [`Block`] is computed internally upon creation
    pub fn new(timestamp: Timestamp, prev_hash: BlockHash, data: BlockData) -> Self {
        let this_hash = hash_block(timestamp, &prev_hash, &data);
        Self { timestamp, prev_hash, this_hash, data }
    }

    /// Returns a reference to the hash of the [`Block`]
    fn hash(&self) -> &BlockHash {
        &self.this_hash
    }

    /// Checks whether the [`Block`] is valid
    ///
    /// To be considered valid, the hash of the [`Block`] must be valid
    pub fn validate(&self) -> bool {
        hash_block(self.timestamp, &self.prev_hash, &self.data) == self.this_hash
    }
	
	const fn size_of_self() -> usize {
		std::mem::size_of::<Self>()
	}
    
    const fn offset_of_timestamp() -> usize {
		std::mem::offset_of!(Block, timestamp)
	}
	
	const fn size_of_timestamp() -> usize {
		std::mem::size_of::<Timestamp>()
	}
    
    const fn offset_of_prev_hash() -> usize {
		std::mem::offset_of!(Block, prev_hash)
	}
	
	const fn size_of_prev_hash() -> usize {
		std::mem::size_of::<BlockHash>()
	}
    
    const fn offset_of_this_hash() -> usize {
		std::mem::offset_of!(Block, this_hash)
	}
	
	const fn size_of_this_hash() -> usize {
		std::mem::size_of::<BlockHash>()
	}
    
    const fn offset_of_data() -> usize {
		std::mem::offset_of!(Block, data)
	}
	
	const fn size_of_data() -> usize {
		std::mem::size_of::<BlockData>()
	}
}

impl bytes_utils::BytesConversion for Block {
	type Error = String;
	
    fn as_boxed_bytes(&self) -> Result<Box<[u8]>, Self::Error> {
        let n_bytes = Self::size_of_self();
        let mut bytes = bytes_utils::boxed_bytes(n_bytes);
        let ref mut bytes_index : usize = 0;
        bytes_utils::append_bytes_raw(&self.timestamp.to_be_bytes(), &mut bytes, bytes_index)?;
        bytes_utils::append_bytes_raw(&self.prev_hash, &mut bytes, bytes_index)?;
        bytes_utils::append_bytes_raw(&self.this_hash, &mut bytes, bytes_index)?;
        bytes_utils::append_bytes_raw(&self.data, &mut bytes, bytes_index)?;
        Ok(bytes)
    }
    
    fn from_raw_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
		if bytes.len() < Self::size_of_self() {
			return Err(format!("Block::from_raw_bytes(): not enough bytes: expected {}, got {}", Self::size_of_self(), bytes.len()));
		}
		let (mut beg, mut end);
		beg = Self::offset_of_timestamp();
		end = beg + Self::size_of_timestamp();
		let timestamp = Timestamp::from_be_bytes(bytes[beg..end].try_into()
			.map_err(|_| String::from("Block::from_raw_bytes(): failed to convert 'timestamp'"))?);
		beg = Self::offset_of_prev_hash();
		end = beg + Self::size_of_prev_hash();
		let prev_hash: BlockHash = bytes[beg..end].try_into()
			.map_err(|_| String::from("Block::from_raw_bytes(): failed to convert 'prev_hash'"))?;
		beg = Self::offset_of_this_hash();
		end = beg + Self::size_of_this_hash();
		let this_hash: BlockHash = bytes[beg..end].try_into()
			.map_err(|_| String::from("Block::from_raw_bytes(): failed to convert 'this_hash"))?;
		beg = Self::offset_of_data();
		end = beg + Self::size_of_data();
		let data: BlockData = bytes[beg..end].try_into()
			.map_err(|_| String::from("Block::from_raw_bytes(): failed to convert 'data"))?;
		Ok(Self{ timestamp, prev_hash, this_hash, data })
	}
}


/// Mines a new `[Block`]
pub fn mine_block(last_block: &Block, data: BlockData) -> Block {
    let timestamp = chrono::Utc::now().timestamp_micros();
    let last_hash = *last_block.hash();

    Block::new(timestamp, last_hash, data)
}

/// Computes the hash of a [`Block`]
fn hash_block(timestamp: Timestamp, last_hash: &[u8; 64], data: &BlockData) -> [u8; 64] {
    let mut hasher = sha2::Sha512::new();
    hasher.update(&timestamp.to_be_bytes());
    hasher.update(last_hash);
    hasher.update(data);
    hasher.finalize().as_slice().try_into().unwrap()
}

/// Validates a [`Block`]
///
/// This function determines whether the `other_block` [`Block`] equals `this_block`.
///
/// The `this_block` [`Block`] is used as a comparison to assess whether `other_block` is valid or
/// not and thus it must be a valid [`Block`] on its own. Using an invalid [`Block`] as `this_block`
/// argument may cause serious havoc.
///
/// This function does not validate the `this_block` [`Block`] per se, which is on the caller
/// to determine. See [`Block::validate()`] to validate a single [`Block`].
pub fn validate_block(this_block: &Block, other_block: &Block) -> ValidationResult {
    if this_block.timestamp != other_block.timestamp {
        return Err(ValidationError::DifferentTimestamp);
    }
    if this_block.data != other_block.data {
        return Err(ValidationError::DifferentData);
    }
    if this_block.prev_hash != other_block.prev_hash {
        return Err(ValidationError::DifferentPreviousHash);
    }
    if this_block.this_hash != other_block.this_hash {
		return Err(ValidationError::DifferentHash);
	}
	Ok(ValidationResponse::ValidBlock)
}

/// Validates a new [`Block`]
///
/// This function determines whether the `new_block` [`Block`] could succeed `last_block` in
/// the blockchain.
///
/// The `last_block` [`Block`] is used as base to assess whether `new_block` is valid or not
/// and thus it must be a valid [`Block`] on its own. Using an invalid [`Block`] as `last_block`
/// argument may cause serious havoc.
///
/// This function does not validate the `last_block` [`Block`] per se, which is on the caller
/// to determine. See [`Block::validate()`] to validate a single [`Block`].
pub fn validate_new_block(last_block: &Block, new_block: &Block) -> ValidationResult {
    if new_block.timestamp <= last_block.timestamp {
        return Err(ValidationError::EarlierTimestamp);
    }
    if new_block.prev_hash != last_block.this_hash {
        return Err(ValidationError::DifferentPreviousHash);
    }
    let computed_hash = hash_block(new_block.timestamp, &last_block.this_hash, &new_block.data);
    if computed_hash != new_block.this_hash {
		return Err(ValidationError::TamperedHash);
	}
	Ok(ValidationResponse::ValidBlock)
}


#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::*;
    
    #[test]
    fn block_struct_layout() {
		let expected_block_size = std::mem::size_of::<Timestamp>() + (std::mem::size_of::<BlockHash>() * 2) + std::mem::size_of::<BlockData>();
		assert_eq!(Block::size_of_self(), expected_block_size);
		let expected_block_alignment: usize = 8;
		assert_eq!(std::mem::align_of::<Block>(), expected_block_alignment);
		let expected_timestamp_offset: usize = 0;
		assert_eq!(Block::offset_of_timestamp(), expected_timestamp_offset);
		let expected_prev_hash_offset = expected_timestamp_offset + std::mem::size_of::<Timestamp>();
		assert_eq!(Block::offset_of_prev_hash(), expected_prev_hash_offset);
		let expected_this_hash_offset = expected_prev_hash_offset + std::mem::size_of::<BlockHash>();
		assert_eq!(Block::offset_of_this_hash(), expected_this_hash_offset);
		let expected_data_offset = expected_this_hash_offset + std::mem::size_of::<BlockHash>();
		assert_eq!(Block::offset_of_data(), expected_data_offset);
	}

    #[test]
    fn generate_genesis_block() {
        let _ = Block::genesis();
    }

    #[test]
    fn mine_new_block() {
        let genesis = Block::genesis();
        let _ = mine_block(&genesis, random_bytes::<BLOCK_DATA_LEN>());
    }

    #[test]
    fn mined_block_is_valid() {
        let genesis = Block::genesis();
        let block = mine_block(&genesis, random_bytes::<BLOCK_DATA_LEN>());
        assert!(validate_new_block(&genesis, &block).is_ok());
    }

    #[test]
    fn tampered_block_is_not_valid() {
        let genesis = Block::genesis();
        {
            let mut tampered_block = mine_block(&genesis, random_bytes::<BLOCK_DATA_LEN>());
            tampered_block.timestamp += 1;
            assert!(!validate_new_block(&genesis, &tampered_block).is_ok());
        }
        {
            let mut tampered_block = mine_block(&genesis, random_bytes::<BLOCK_DATA_LEN>());
            tampered_block.prev_hash[0] += 1;
            assert!(!validate_new_block(&genesis, &tampered_block).is_ok());
        }
        {
            let mut tampered_block = mine_block(&genesis, random_bytes::<BLOCK_DATA_LEN>());
            tampered_block.this_hash[0] += 1;
            assert!(!validate_new_block(&genesis, &tampered_block).is_ok());
        }
        {
            let mut tampered_block = mine_block(&genesis, random_bytes::<BLOCK_DATA_LEN>());
            tampered_block.data[0] += 1;
            assert!(!validate_new_block(&genesis, &tampered_block).is_ok());
        }
    }
    
    #[test]
    fn bytes_conversion() {
		use bytes_utils::BytesConversion;
		let original_block = Block::new(random_timestamp(), random_bytes::<BLOCK_HASH_LEN>(), random_bytes::<BLOCK_DATA_LEN>());
		let block_bytes = original_block.as_actix_bytes().unwrap();
		let converted_block = Block::from_actix_bytes(block_bytes).unwrap();
		assert_eq!(original_block, converted_block);
	}
}

