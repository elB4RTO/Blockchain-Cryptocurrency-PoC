use crate::keys;
use crate::transaction;

use bytes_utils::BytesConversion;

use ed25519_dalek::{PUBLIC_KEY_LENGTH, SigningKey, VerifyingKey};


pub type WalletKeys = SigningKey;
pub type WalletPublicKey = VerifyingKey;
pub type WalletBalance = transaction::Amount;
pub type WalletAddress = [u8; PUBLIC_KEY_LENGTH];


const WALLET_BALANCE_BYTES_LENGTH: usize = std::mem::size_of::<WalletBalance>();
const PUBLIC_WALLET_BYTES_LENGTH: usize = WALLET_BALANCE_BYTES_LENGTH + PUBLIC_KEY_LENGTH;


pub trait Wallet {
	fn balance(&self) -> WalletBalance;
	
	fn address(&self) -> WalletAddress;
}


pub struct PrivateWallet {
	balance: WalletBalance,
	keys_pair: WalletKeys,
}

impl PrivateWallet {
	fn generate() -> Self {
		Self {
			balance: 0,
			keys_pair: keys::generate_keys_pair(),
		}
	}
	
	pub fn new() -> (Self, transaction::SignedTransaction) {
		let wallet = Self::generate();
		let transaction = transaction::Transaction::new(0, wallet.to_public(), wallet.to_public())
			.into_signed(&wallet.keys_pair)
			.unwrap();
		(wallet, transaction)
	}
	
	pub(crate) fn keys_pair(&self) -> &WalletKeys {
		&self.keys_pair
	}
	
	pub fn make_transaction<'a>(&'a mut self, amount: transaction::Amount, receiver: &'a mut PublicWallet) -> Result<transaction::UncommittedTransaction<'a>, String> {
		if amount == 0 {
			return Err(String::from("PrivateWallet::make_transaction(): invalid amount: amount is zero"));
		} else if amount > self.balance {
			return Err(String::from("PrivateWallet::make_transaction(): invalid amount: amount is higher than balance"));
		}
		if self.to_public() == *receiver {
			return Err(String::from("PrivateWallet::make_transaction(): invalid transaction: receiver is same as sender"));
		}
		Ok(transaction::UncommittedTransaction::new(amount, self, receiver))
	}
	
	pub(crate) fn check_transaction(&self, transaction: &transaction::Transaction) -> Result<(), String> {
		let transaction_amount = transaction.amount();
		if self.balance < transaction_amount {
			return Err(String::from("PrivateWallet::check_transaction(): invalid transaction: transaction amount is higher than wallet balance"));
		} else if transaction.sender_balance() != self.balance {
			return Err(String::from("PrivateWallet::check_transaction(): invalid transaction: sender balance does not match wallet balance"));
		} else if transaction.sender_address() != self.address() {
			return Err(String::from("PrivateWallet::check_transaction(): invalid transaction: sender address does not match wallet address"));
		}
		Ok(())
	}
	
	pub(crate) fn commit_transaction(&mut self, transaction: transaction::Transaction) -> Result<transaction::SignedTransaction, String> {
		let transaction_amount = transaction.amount();
		let transaction_bytes = transaction.as_boxed_bytes()
			.map_err(|e| e + "\nPrivateWallet::commit_transaction(): failed to commit transaction")?;
		let signature = keys::sign(&self.keys_pair, &transaction_bytes);
		let signed_transaction = transaction::SignedTransaction::new(transaction, signature);
		self.balance -= transaction_amount;
		Ok(signed_transaction)
	}
	
	pub fn to_public(&self) -> PublicWallet {
		PublicWallet::new(self.balance, self.address())
	}
}

#[cfg(test)]
impl PrivateWallet {
	pub(crate) fn with_balance(mut self, balance: WalletBalance) -> Self {
		self.balance = balance;
		self
	}
	
	pub(crate) fn with_keys(mut self, keys_pair: WalletKeys) -> Self {
		self.keys_pair = keys_pair;
		self
	}
}

impl Wallet for PrivateWallet {
	fn balance(&self) -> WalletBalance {
		self.balance
	}
	
	fn address(&self) -> WalletAddress {
		self.keys_pair.verifying_key().to_bytes()
	}
}


#[derive(Clone, PartialEq)]
pub struct PublicWallet {
	balance: WalletBalance,
	address: WalletAddress,
}

impl PublicWallet {
	pub fn new(balance: WalletBalance, address: WalletAddress) -> Self {
		Self { balance, address }
	}
	
	pub(crate) fn check_transaction(&self, transaction: &transaction::Transaction) -> Result<(), String> {
		if transaction.receiver_balance() != self.balance {
			return Err(String::from("PublicWallet::commit_transaction(): invalid transaction: receiver balance does not match wallet balance"));
		} else if transaction.receiver_address() != self.address {
			return Err(String::from("PublicWallet::commit_transaction(): invalid transaction: receiver address does not match wallet address"));
		}
		Ok(())
	}
	
	pub(crate) fn commit_transaction(&mut self, transaction_amount: transaction::Amount) {
		self.balance += transaction_amount;
	}
	
	pub(crate) fn to_bytes_array(&self) -> [u8; PUBLIC_WALLET_BYTES_LENGTH] {
		let mut bytes: [u8; PUBLIC_WALLET_BYTES_LENGTH] = [0; PUBLIC_WALLET_BYTES_LENGTH];
		
		(&mut bytes[..WALLET_BALANCE_BYTES_LENGTH]).copy_from_slice(&self.balance.to_be_bytes());
		(&mut bytes[WALLET_BALANCE_BYTES_LENGTH..]).copy_from_slice(&self.address());
		
		bytes
	}
}

impl Wallet for PublicWallet {
	fn balance(&self) -> WalletBalance {
		self.balance
	}
	
	fn address(&self) -> WalletAddress {
		self.address
	}
}

impl bytes_utils::BytesConversion for PublicWallet {
	type Error = String;
	
    fn as_boxed_bytes(&self) -> Result<Box<[u8]>, Self::Error> {
        let mut bytes = bytes_utils::boxed_bytes(PUBLIC_WALLET_BYTES_LENGTH);
        let ref mut bytes_index : usize = 0;
		
		bytes_utils::append_bytes_raw(&self.balance.to_be_bytes(), &mut bytes, bytes_index)?;
		bytes_utils::append_bytes_raw(&self.address(), &mut bytes, bytes_index)?;
        
        Ok(bytes)
    }
    
    fn from_raw_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
		if bytes.len() < PUBLIC_WALLET_BYTES_LENGTH {
			return Err(format!("PublicWallet::from_raw_bytes(): not enough bytes: expected {}, got {}", PUBLIC_WALLET_BYTES_LENGTH, bytes.len()));
		}
		
		let (mut beg, mut end) = (0, WALLET_BALANCE_BYTES_LENGTH);
		let balance = WalletBalance::from_be_bytes(bytes[beg..end].try_into()
			.map_err(|_| String::from("PublicWallet::from_raw_bytes(): failed to convert balance"))?);
		(beg, end) = (end, end + PUBLIC_KEY_LENGTH);
		let address = bytes[beg..end].try_into()
			.map_err(|_| String::from("PublicWallet::from_raw_bytes(): failed to convert address"))?;
		
		Ok(Self::new(balance, address))
	}
}


#[cfg(test)]
mod test {
	use super::*;
	use crate::test_utils::*;
	
	#[test]
	fn create_private_wallet() {
		let _ = PrivateWallet::generate();
		let (_,_) = PrivateWallet::new();
	}
	
	#[test]
	fn create_public_wallet() {
		let _ = PublicWallet::new(0, [0; PUBLIC_KEY_LENGTH]);
		let _ = PrivateWallet::generate().to_public();
	}
	
	#[test]
	fn make_valid_transaction() {
		let mut sender_private_wallet = new_private_wallet(100);
		let mut receiver_public_wallet = new_public_wallet(100);
		
		assert!(sender_private_wallet.make_transaction(1, &mut receiver_public_wallet).is_ok());
		assert!(sender_private_wallet.make_transaction(100, &mut receiver_public_wallet).is_ok());
	}
	
	#[test]
	fn make_invalid_transactions() {
		let mut sender_private_wallet = new_private_wallet(100);
		let mut receiver_public_wallet = new_public_wallet(100);
		
		assert!(sender_private_wallet.make_transaction(0, &mut receiver_public_wallet).is_err());
		assert!(sender_private_wallet.make_transaction(101, &mut receiver_public_wallet).is_err());
	}
}
