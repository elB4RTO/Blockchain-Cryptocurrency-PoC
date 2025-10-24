use crate::keys;
use crate::wallet::*;

use bytes_utils::BytesConversion;

use sha2::Digest;


pub type Timestamp = i64;
pub type Amount = u128;
pub type Signature = ed25519_dalek::Signature;

type TransactionHash = [u8; TRANSACTION_HASH_BYTES_LENGTH];
type TransactionId = [u8; TRANSACTION_ID_BYTES_LENGTH];
type TransactionBytes = [u8; TRANSACTION_BYTES_LENGTH];
type SignatureBytes = [u8; SIGNATURE_BYTES_LENGTH];
type SignedTransactionBytes = [u8; SIGNED_TRANSACTION_BYTES_LENGTH];


const TRANSACTION_HASH_BYTES_LENGTH: usize = 64;
const TRANSACTION_TIMESTAMP_BYTES_LENGTH: usize = std::mem::size_of::<Timestamp>();
const TRANSACTION_AMOUNT_BYTES_LENGTH: usize = std::mem::size_of::<Amount>();
const TRANSACTION_ADDRESS_BYTES_LENGTH: usize = std::mem::size_of::<WalletAddress>();
const TRANSACTION_ID_BYTES_LENGTH: usize = 8;
const TRANSACTION_BYTES_LENGTH: usize = TRANSACTION_HASH_BYTES_LENGTH + TRANSACTION_TIMESTAMP_BYTES_LENGTH + (3 * TRANSACTION_AMOUNT_BYTES_LENGTH) + (2 * TRANSACTION_ADDRESS_BYTES_LENGTH) + TRANSACTION_ID_BYTES_LENGTH;
const SIGNATURE_BYTES_LENGTH: usize = Signature::BYTE_SIZE;
const SIGNED_TRANSACTION_BYTES_LENGTH: usize = TRANSACTION_BYTES_LENGTH + SIGNATURE_BYTES_LENGTH;


pub struct UncommittedTransaction<'a> {
	amount: Amount,
	sender: &'a mut PrivateWallet,
	receiver: &'a mut PublicWallet,
}

impl<'a> UncommittedTransaction<'a> {
	pub(crate) fn new(amount: Amount, sender: &'a mut PrivateWallet, receiver: &'a mut PublicWallet) -> Self {
		Self { amount, sender, receiver }
	}
	
	pub fn amount(&self) -> Amount {
		self.amount
	}
	
	pub fn sender_address(&self) -> WalletAddress {
		self.sender.address()
	}
	
	pub fn sender_balance(&self) -> WalletBalance {
		self.sender.balance()
	}
	
	pub fn final_sender_balance(&self) -> WalletBalance {
		self.sender.balance() - self.amount
	}
	
	pub fn receiver_address(&self) -> WalletAddress {
		self.receiver.address()
	}
	
	pub fn receiver_balance(&self) -> WalletBalance {
		self.receiver.balance()
	}
	
	pub fn final_receiver_balance(&self) -> WalletBalance {
		self.receiver.balance() + self.amount
	}
	
	pub fn finalize(self) -> Result<SignedTransaction, String> {
		let transaction = Transaction::new(self.amount(), self.sender.to_public(), self.receiver.clone());
		self.sender.check_transaction(&transaction)?;
		self.receiver.check_transaction(&transaction)?;
		let signed_transaction = self.sender.commit_transaction(transaction)?;
		self.receiver.commit_transaction(self.amount());
		Ok(signed_transaction)
	}
}


#[derive(PartialEq)]
pub(crate) struct Transaction {
	timestamp: Timestamp,
	amount: Amount,
	sender: PublicWallet,
	receiver: PublicWallet,
}

impl Transaction {
	pub(crate) fn new(amount: Amount, sender: PublicWallet, receiver: PublicWallet) -> Self {
		let timestamp = chrono::Utc::now().timestamp_micros();
		
		Self { timestamp, amount, sender, receiver }
	}
	
	pub(crate) fn amount(&self) -> Amount {
		self.amount
	}
	
	pub(crate) fn sender_address(&self) -> WalletAddress {
		self.sender.address()
	}
	
	pub(crate) fn sender_balance(&self) -> WalletBalance {
		self.sender.balance()
	}
	
	pub(crate) fn receiver_address(&self) -> WalletAddress {
		self.receiver.address()
	}
	
	pub(crate) fn receiver_balance(&self) -> WalletBalance {
		self.receiver.balance()
	}
	
	fn hash(&self) -> TransactionHash {
		let mut hasher = sha2::Sha512::new();
		hasher.update(&self.amount.to_be_bytes());
		hasher.update(&self.sender.to_bytes_array());
		hasher.update(&self.receiver.to_bytes_array());
		hasher.update(&self.timestamp.to_be_bytes());
		hasher.finalize().as_slice().try_into().unwrap()
	}
	
	fn updated_sender_balance(&self) -> WalletBalance {
		self.sender.balance() - self.amount
	}
	
	fn updated_receiver_balance(&self) -> WalletBalance {
		self.receiver.balance() + self.amount
	}
	
	fn final_id(bytes: &[u8]) -> TransactionId {
		let mut hasher = sha2::Sha512::new();
		hasher.update(bytes);
		let full_hash: TransactionHash = hasher.finalize().as_slice().try_into().unwrap();
		full_hash[..8].try_into().unwrap()
	}
	
	pub fn into_signed(self, keys_pair: &WalletKeys) -> Result<SignedTransaction, String> {
		let bytes = self.as_boxed_bytes()?;
		let signature = keys::sign(keys_pair, &bytes);
		Ok(SignedTransaction::new(self, signature))
	}
}

impl bytes_utils::BytesConversion for Transaction {
	type Error = String;
	
    fn as_boxed_bytes(&self) -> Result<Box<[u8]>, Self::Error> {
        let mut bytes = bytes_utils::boxed_bytes(TRANSACTION_BYTES_LENGTH);
        let ref mut bytes_index : usize = 0;
		
		bytes_utils::append_bytes_raw(&self.hash(), &mut bytes, bytes_index)?;
		bytes_utils::append_bytes_raw(&self.timestamp.to_be_bytes(), &mut bytes, bytes_index)?;
		bytes_utils::append_bytes_raw(&self.amount.to_be_bytes(), &mut bytes, bytes_index)?;
		bytes_utils::append_bytes_raw(&self.sender.address(), &mut bytes, bytes_index)?;
		bytes_utils::append_bytes_raw(&self.updated_sender_balance().to_be_bytes(), &mut bytes, bytes_index)?;
		bytes_utils::append_bytes_raw(&self.receiver.address(), &mut bytes, bytes_index)?;
		bytes_utils::append_bytes_raw(&self.updated_receiver_balance().to_be_bytes(), &mut bytes, bytes_index)?;
		let transaction_id = Self::final_id(&bytes[..]);
		bytes_utils::append_bytes_raw(&transaction_id, &mut bytes, bytes_index)?;
        
        Ok(bytes)
    }
    
    fn from_raw_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
		if bytes.len() < TRANSACTION_BYTES_LENGTH {
			return Err(format!("Transaction::from_raw_bytes(): not enough bytes: expected {}, got {}", TRANSACTION_BYTES_LENGTH, bytes.len()));
		}
		
		let (mut beg, mut end) = (0, TRANSACTION_HASH_BYTES_LENGTH);
		let transaction_hash = &bytes[beg..end];
		(beg, end) = (end, end + TRANSACTION_TIMESTAMP_BYTES_LENGTH);
		let timestamp = Timestamp::from_be_bytes(bytes[beg..end].try_into()
			.map_err(|_| String::from("Transaction::from_raw_bytes(): failed to convert timestamp"))?);
		(beg, end) = (end, end + TRANSACTION_AMOUNT_BYTES_LENGTH);
		let amount = Amount::from_be_bytes(bytes[beg..end].try_into()
			.map_err(|_| String::from("Transaction::from_raw_bytes(): failed to convert amount"))?);
		(beg, end) = (end, end + TRANSACTION_ADDRESS_BYTES_LENGTH);
		let sender_address: WalletAddress = bytes[beg..end].try_into()
			.map_err(|_| String::from("Transaction::from_raw_bytes(): failed to convert sender address"))?;
		(beg, end) = (end, end + TRANSACTION_AMOUNT_BYTES_LENGTH);
		let original_sender_balance = WalletBalance::from_be_bytes(bytes[beg..end].try_into()
			.map_err(|_| String::from("Transaction::from_raw_bytes(): failed to convert sender balance"))?) + amount;
		(beg, end) = (end, end + TRANSACTION_ADDRESS_BYTES_LENGTH);
		let receiver_address: WalletAddress = bytes[beg..end].try_into()
			.map_err(|_| String::from("Transaction::from_raw_bytes(): failed to convert receiver address"))?;
		(beg, end) = (end, end + TRANSACTION_AMOUNT_BYTES_LENGTH);
		let original_receiver_balance = WalletBalance::from_be_bytes(bytes[beg..end].try_into()
			.map_err(|_| String::from("Transaction::from_raw_bytes(): failed to convert receiver balance"))?) - amount;
		(beg, end) = (end, end + TRANSACTION_ID_BYTES_LENGTH);
		let transaction_id = &bytes[beg..end];
		
		let sender = PublicWallet::new(original_sender_balance, sender_address);
		let receiver = PublicWallet::new(original_receiver_balance, receiver_address);
		
		let transaction = Self { timestamp, amount, sender, receiver };
		
		if transaction.hash() != *transaction_hash {
			return Err(String::from("Transaction::from_bytes(): transaction hash does not match"));
		}
		
		let mut bytes_without_id: TransactionBytes = [0; TRANSACTION_BYTES_LENGTH];
		(beg, end) = (0, TRANSACTION_BYTES_LENGTH - TRANSACTION_ID_BYTES_LENGTH);
		bytes_without_id[beg..end].copy_from_slice(&bytes[beg..end]);
		
		if Self::final_id(&bytes_without_id) != *transaction_id {
			return Err(String::from("Transaction::from_bytes(): transaction ID does not match"));
		}
		
		Ok(transaction)
	}
}


#[derive(PartialEq)]
pub struct SignedTransaction {
	transaction: Transaction,
	signature: Signature,
}

impl SignedTransaction {
	pub(crate) fn new(transaction: Transaction, signature: Signature) -> Self {
		Self { transaction, signature }
	}
	
	pub fn amount(&self) -> Amount {
		self.transaction.amount()
	}
	
	pub fn sender_address(&self) -> WalletAddress {
		self.transaction.sender_address()
	}
	
	pub fn sender_balance(&self) -> WalletBalance {
		self.transaction.sender_balance()
	}
	
	pub fn receiver_address(&self) -> WalletAddress {
		self.transaction.receiver_address()
	}
	
	pub fn receiver_balance(&self) -> WalletBalance {
		self.transaction.receiver_balance()
	}
	
	pub fn verify(&self) -> Result<bool, String> {
		let public_key = WalletPublicKey::from_bytes(&self.transaction.sender.address())
			.map_err(|_| String::from("SignedTransaction::verify(): failed to convert public key"))?;
		Ok(crate::keys::verify_public(&public_key, &self.signature, &self.transaction.as_boxed_bytes()?))
	}
}

impl bytes_utils::BytesConversion for SignedTransaction {
	type Error = String;
	
    fn as_boxed_bytes(&self) -> Result<Box<[u8]>, Self::Error> {
        let mut bytes = bytes_utils::boxed_bytes(SIGNED_TRANSACTION_BYTES_LENGTH);
        let ref mut bytes_index : usize = 0;
		
		bytes_utils::append_bytes_raw(&self.transaction.as_boxed_bytes()?, &mut bytes, bytes_index)?;
		bytes_utils::append_bytes_raw(&self.signature.to_bytes(), &mut bytes, bytes_index)?;
        
        Ok(bytes)
    }
    
    fn from_raw_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
		if bytes.len() < SIGNED_TRANSACTION_BYTES_LENGTH {
			return Err(format!("SignedTransaction::from_raw_bytes(): not enough bytes: expected {}, got {}", SIGNED_TRANSACTION_BYTES_LENGTH, bytes.len()));
		}
		
		let (mut beg, mut end) = (0, TRANSACTION_BYTES_LENGTH);
		let transaction = Transaction::from_raw_bytes(&bytes[beg..end])
			.map_err(|e| e + "\nSignedTransaction::from_raw_bytes(): failed to convert transaction")?;
		(beg, end) = (end, end + SIGNATURE_BYTES_LENGTH);
		let signature = Signature::from_slice(&bytes[beg..end])
			.map_err(|_| String::from("SignedTransaction::from_raw_bytes(): failed to convert signature"))?;
		
		let signed_transaction = Self { transaction, signature };
		match signed_transaction.verify() {
			Ok(true) => (),
			Ok(false) => return Err(String::from("SignedTransaction::from_raw_bytes(): failed to verify")),
			Err(e) => return Err(e + "\nSignedTransaction::from_raw_bytes(): failed to verify"),
		}
		
		Ok(signed_transaction)
	}
}


#[cfg(test)]
mod test {
	use super::*;
	use crate::test_utils::*;
	use crate::keys::generate_keys_pair;
	
	#[test]
	fn transaction_to_bytes() {
		let amount = 50;
		let sender = new_public_wallet(100);
		let receiver = new_public_wallet(100);
		let transaction = Transaction::new(amount, sender, receiver);
		
		assert!(transaction.as_boxed_bytes().is_ok());
	}
	
	#[test]
	fn transaction_from_bytes() {
		let amount = 50;
		let sender = new_public_wallet(100);
		let receiver = new_public_wallet(100);
		let transaction = Transaction::new(amount, sender, receiver);
		let bytes = transaction.as_boxed_bytes().unwrap();
		
		let recovered_transaction = Transaction::from_raw_bytes(&bytes).unwrap();
		assert!(recovered_transaction == transaction);
	}
	
	#[test]
	fn create_signed_transaction() {
		let keys_pair = generate_keys_pair();
		let sender = custom_public_wallet(100, keys_pair.clone());
		let receiver = new_public_wallet(100);
		let transaction = Transaction::new(50, sender, receiver);
		
		assert!(transaction.into_signed(&keys_pair).is_ok());
	}
	
	#[test]
	fn verify_signed_transaction() {
		let keys_pair = generate_keys_pair();
		let sender = custom_public_wallet(100, keys_pair.clone());
		let receiver = new_public_wallet(100);
		let transaction = Transaction::new(50, sender, receiver);
		let signed_transaction = transaction.into_signed(&keys_pair).unwrap();
		
		assert!(signed_transaction.verify().is_ok());
		assert!(signed_transaction.verify().unwrap());
	}
	
	#[test]
	fn signed_transaction_to_bytes() {
		let keys_pair = generate_keys_pair();
		let sender = custom_public_wallet(100, keys_pair.clone());
		let receiver = new_public_wallet(100);
		let transaction = Transaction::new(50, sender, receiver);
		let signed_transaction = transaction.into_signed(&keys_pair).unwrap();
		
		assert!(signed_transaction.as_boxed_bytes().is_ok());
	}
	
	#[test]
	fn signed_transaction_from_bytes() {
		let keys_pair = generate_keys_pair();
		let sender = custom_public_wallet(100, keys_pair.clone());
		let receiver = new_public_wallet(100);
		let transaction = Transaction::new(50, sender, receiver);
		let signed_transaction = transaction.into_signed(&keys_pair).unwrap();
		let bytes = signed_transaction.as_boxed_bytes().unwrap();
		
		let recovered_signed_transaction = SignedTransaction::from_raw_bytes(&bytes).unwrap();
		assert!(recovered_signed_transaction == signed_transaction);
	}
}
