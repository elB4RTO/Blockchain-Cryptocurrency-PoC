use crate::block::*;


/// Result type returned on [`Blockchain`] validation
pub type ValidationResult = Result<ValidationResponse, ValidationError>;

/// The [`Ok`] type of a [`ValidationResult`]
pub enum ValidationResponse {
    EqualBlockchain,
    LongerBlockchain,
}

/// The [`Err`] type of a [`ValidationResult`]
#[derive(Debug)]
pub enum ValidationError {
    DifferentGenesis,
    DifferentBlocks,
    TamperedBlocks,
    ShorterBlockchain,
}


/// Represents a blockchain
pub struct Blockchain {
    /// The actual blockchain
    blocks: Vec<Block>,
}

impl Blockchain {
    /// Creates a new invalid [`Blockchain`] with no [`Block`]s
    pub const fn invalid() -> Self {
        Self {
            blocks: Vec::new(),
        }
    }

    /// Creates a new [`Blockchain`] with only the genesis [`Block`]
    pub fn new() -> Self {
        Self {
            blocks: Vec::from([Block::genesis()]),
        }
    }

    /// Adds a new existing [`Block`]
    pub fn add_block(&mut self, block: Block) {
        self.blocks.push(block);
    }

    /// Mines a new [`Block`] using the given data
    pub fn mine_block(&mut self, data: BlockData) {
        let block = mine_block(self.last_block(), data);
        self.add_block(block);
    }

    /// Returns a reference to the genesis [`Block`]
    fn genesis_block(&self) -> &Block {
        self.blocks.first().unwrap()
    }

    /// Returns a reference to the last [`Block`] in the chain
    pub fn last_block(&self) -> &Block {
        self.blocks.last().unwrap()
    }

    /// Returns the number of [`Block`]s stored in the [`Blockchain`]
    pub fn blocks_count(&self) -> usize {
        self.blocks.len()
    }

    /// Checks whether the [`Blockchain`] is valid
    ///
    /// To be considered valid, the genesis [`Block`] must be equal the default genesis [`Block`]
    /// and all the following [`Block`]s must be valid (see [`validate_new_block()`]).
    pub fn validate(&self) -> bool {
        self.genesis_block() == &Block::genesis() && self.iter().all(|(pb, cb)| validate_new_block(pb, cb).is_ok())
    }

    /// Attempts to update the [`Blockchain`]
    ///
    /// The new [`Blockchain`] is validated prior to committing the update.
    pub fn update(&mut self, new_blockchain: Blockchain) -> Result<(), ValidationError> {
        match validate_blockchain(self, &new_blockchain)? {
            ValidationResponse::EqualBlockchain => (),
            ValidationResponse::LongerBlockchain => self.blocks = new_blockchain.blocks,
        }
        Ok(())
    }

    /// Creates a [`BlockchainIterator`] upon the [`Blockchain`]
    fn iter(&self) -> BlockchainIterator {
        BlockchainIterator::new(self)
    }

    /// Creates a [`BlockchainIterator`] upon the blockchain, starting from the given index
    fn iter_from(&self, block_index: usize) -> BlockchainIterator {
        BlockchainIterator::new(self).with_index(block_index)
    }
}

impl bytes_utils::BytesConversion for Blockchain {
	type Error = String;
	
    fn as_boxed_bytes(&self) -> Result<Box<[u8]>, Self::Error> {
        let n_bytes = std::mem::size_of::<u64>() + (self.blocks.len() * std::mem::size_of::<Block>());
        let mut bytes = bytes_utils::boxed_bytes(n_bytes);
        let ref mut bytes_index: usize = 0;
        bytes_utils::append_bytes_raw(&self.blocks.len().to_be_bytes(), &mut bytes, bytes_index)?;
        for block in self.blocks.iter() {
            bytes_utils::append_bytes_raw(&block.as_boxed_bytes()?, &mut bytes, bytes_index)?;
        }
        Ok(bytes)
    }
    
    fn from_raw_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
		let mut blockchain = Self::invalid();
		let (mut beg, mut end);
		beg = 0;
		end = std::mem::size_of::<usize>();
		let vec_len = usize::from_be_bytes(bytes[beg..end].try_into()
			.map_err(|_| String::from("Blockchain::from_raw_bytes(): failed to convert blocks length"))?);
		blockchain.blocks.reserve(vec_len);
		for _ in 0..vec_len {
			beg = end;
			end += std::mem::size_of::<Block>();
			blockchain.blocks.push(Block::from_raw_bytes(&bytes[beg..end])?);
		}
		Ok(blockchain)
	}
}


/// An iterator over a [`Blockchain`]
///
/// Calling [`Iterator::next`] returns a tuple of 2 blocks referencing the previous
/// and the current blocks in the iteration.
struct BlockchainIterator<'a> {
    bc: &'a Blockchain,
    i: usize,
}

impl<'a> BlockchainIterator<'a> {
    /// Creates a new [`BlockchainIterator`]
    fn new(bc: &'a Blockchain) -> Self {
        Self { bc, i: 1 }
    }

    /// Replaces the current index with the given one
    fn with_index(mut self, i: usize) -> Self {
        assert!(i > 0);
        self.i = i;
        self
    }
}

impl<'a> Iterator for BlockchainIterator<'a> {
    type Item = (&'a Block, &'a Block);

    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.bc.blocks_count() {
            return None;
        }
        let ret = Some((&self.bc.blocks[self.i-1], &self.bc.blocks[self.i]));
        self.i += 1;
        ret
    }
}


/// Validates a [`Blockchain`] using another blockchain as model
///
/// # Beware
///
/// The `this` [`Blockchain`] is used as base to assess whether `other` is valid or not
/// and thus it must be a valid [`Blockchain`] on its own. Using an invalid [`Blockchain`]
/// as `this` argument may cause serious havoc.
///
/// This function does not validate the `this` [`Blockchain`] per se, which is on the caller
/// to determine. See [`Blockchain::validate()`] to validate a single [`Blockchain`].
pub(crate) fn validate_blockchain(this: &Blockchain, other: &Blockchain) -> ValidationResult {
    if this.blocks_count() > other.blocks_count() {
        return Err(ValidationError::ShorterBlockchain);
    }
    if other.genesis_block() != &Block::genesis() {
        return Err(ValidationError::DifferentGenesis);
    }
    for (this_blocks, other_blocks) in this.iter().zip(other.iter()) {
        if this_blocks != other_blocks {
            return Err(ValidationError::DifferentBlocks);
        }
    }
    if this.blocks_count() == other.blocks_count() {
        return Ok(ValidationResponse::EqualBlockchain);
    }
    if validate_new_block(this.last_block(), &other.blocks[this.blocks_count()]).is_err() {
        return Err(ValidationError::TamperedBlocks);
    }
    for (other_last_block, other_next_block) in other.iter_from(this.blocks_count()) {
        if validate_new_block(other_last_block, other_next_block).is_err() {
            return Err(ValidationError::TamperedBlocks);
        }
    }
    Ok(ValidationResponse::LongerBlockchain)
}


#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::*;

    #[test]
    fn first_block_is_genesis() {
        assert_eq!(Blockchain::new().blocks[0], Block::genesis());
    }

    #[test]
    fn new_blockchain_is_valid() {
        assert!(Blockchain::new().validate());
    }

    #[test]
    fn blockchain_with_blocks_is_valid() {
        let mut bc = Blockchain::new();
        bc.add_block(random_block(bc.last_block()));
        bc.add_block(random_block(bc.last_block()));
        bc.add_block(random_block(bc.last_block()));
        assert!(bc.validate());
    }

    #[test]
    fn blockchain_with_tampered_blocks_is_not_valid() {
        let mut bc = Blockchain::new();
        bc.add_block(random_block(bc.last_block()));
        bc.add_block(random_block(bc.last_block()));
        bc.blocks[1] = fully_random_block();
        assert!(!bc.validate());
    }

    #[test]
    fn new_blockchain_vs_new_blockchain_is_valid() {
        assert!(validate_blockchain(&Blockchain::new(), &Blockchain::new()).is_ok());
    }

    #[test]
    fn blockchain_with_blocks_vs_new_blockchain_is_not_valid() {
        let mut this_bc = Blockchain::new();
        this_bc.add_block(random_block(this_bc.last_block()));
        this_bc.add_block(random_block(this_bc.last_block()));
        this_bc.add_block(random_block(this_bc.last_block()));
        let other_bc = Blockchain::new();
        assert!(validate_blockchain(&this_bc, &other_bc).is_err());
    }

    #[test]
    fn new_blockchain_vs_blockchain_with_blocks_is_valid() {
        let this_bc = Blockchain::new();
        let mut other_bc = Blockchain::new();
        other_bc.add_block(random_block(other_bc.last_block()));
        other_bc.add_block(random_block(other_bc.last_block()));
        other_bc.add_block(random_block(other_bc.last_block()));
        assert!(validate_blockchain(&this_bc, &other_bc).is_ok());
    }

    #[test]
    fn blockchain_with_blocks_vs_blockchain_with_same_blocks_is_valid() {
        let mut this_bc = Blockchain::new();
        this_bc.add_block(random_block(this_bc.last_block()));
        this_bc.add_block(random_block(this_bc.last_block()));
        this_bc.add_block(random_block(this_bc.last_block()));
        let mut other_bc = Blockchain::new();
        other_bc.blocks = this_bc.blocks.clone();
        assert!(validate_blockchain(&this_bc, &other_bc).is_ok());
    }

    #[test]
    fn blockchain_with_blocks_vs_blockchain_with_different_blocks_is_not_valid() {
        let mut this_bc = Blockchain::new();
        this_bc.add_block(random_block(this_bc.last_block()));
        this_bc.add_block(random_block(this_bc.last_block()));
        let mut other_bc = Blockchain::new();
        other_bc.blocks = this_bc.blocks.clone();
        this_bc.add_block(random_block(this_bc.last_block()));
        other_bc.add_block(random_block(other_bc.last_block()));
        assert!(validate_blockchain(&this_bc, &other_bc).is_err());
    }

    #[test]
    fn update_with_equal_blockchain_is_ok() {
        let mut this_bc = Blockchain::new();
        this_bc.add_block(random_block(this_bc.last_block()));
        this_bc.add_block(random_block(this_bc.last_block()));
        this_bc.add_block(random_block(this_bc.last_block()));
        let mut other_bc = Blockchain::new();
        other_bc.blocks = this_bc.blocks.clone();
        assert!(this_bc.update(other_bc).is_ok());
    }

    #[test]
    fn update_with_longer_blockchain_is_ok() {
        let mut this_bc = Blockchain::new();
        this_bc.add_block(random_block(this_bc.last_block()));
        this_bc.add_block(random_block(this_bc.last_block()));
        this_bc.add_block(random_block(this_bc.last_block()));
        let mut other_bc = Blockchain::new();
        other_bc.blocks = this_bc.blocks.clone();
        other_bc.add_block(random_block(other_bc.last_block()));
        assert!(this_bc.update(other_bc).is_ok());
    }

    #[test]
    fn update_with_shorter_blockchain_is_not_ok() {
        let mut this_bc = Blockchain::new();
        this_bc.add_block(random_block(this_bc.last_block()));
        this_bc.add_block(random_block(this_bc.last_block()));
        this_bc.add_block(random_block(this_bc.last_block()));
        let mut other_bc = Blockchain::new();
        other_bc.blocks = this_bc.blocks.clone();
        other_bc.blocks.truncate(this_bc.blocks.len() - 1);
        assert!(this_bc.update(other_bc).is_err());
    }

    #[test]
    fn update_with_different_blockchain_is_not_ok() {
        let mut this_bc = Blockchain::new();
        this_bc.add_block(random_block(this_bc.last_block()));
        this_bc.add_block(random_block(this_bc.last_block()));
        this_bc.add_block(random_block(this_bc.last_block()));
        {
            let mut other_bc = Blockchain::new();
            other_bc.blocks = this_bc.blocks.clone();
            other_bc.blocks[0] = Block::new(random_timestamp(), random_bytes::<{BLOCK_HASH_LEN}>(), random_bytes::<{BLOCK_DATA_LEN}>());
            assert!(this_bc.update(other_bc).is_err());
        }
        {
            let mut other_bc = Blockchain::new();
            other_bc.blocks = this_bc.blocks.clone();
            this_bc.add_block(random_block(this_bc.last_block()));
            other_bc.add_block(random_block(other_bc.last_block()));
            assert!(this_bc.update(other_bc).is_err());
        }
    }
    
    #[test]
    fn bytes_conversion() {
		use bytes_utils::BytesConversion;
		let mut original_bc = Blockchain::new();
		original_bc.add_block(random_block(original_bc.last_block()));
		original_bc.add_block(random_block(original_bc.last_block()));
		original_bc.add_block(random_block(original_bc.last_block()));
		let bc_bytes = original_bc.as_actix_bytes().unwrap();
		let converted_bc = Blockchain::from_actix_bytes(bc_bytes).unwrap();
		assert_eq!(original_bc.blocks, converted_bc.blocks);
	}
}

