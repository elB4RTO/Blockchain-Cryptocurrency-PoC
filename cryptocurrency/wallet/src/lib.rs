mod keys;
pub mod transaction;
pub mod wallet;


pub use transaction::{SignedTransaction, UncommittedTransaction};
pub use wallet::*;


#[cfg(test)]
pub(crate) mod test_utils {
	use super::*;
	
	pub(crate) fn new_private_wallet(balance: WalletBalance) -> PrivateWallet {
		let (wallet, _) = PrivateWallet::new();
		wallet.with_balance(balance)
	}
	
	pub(crate) fn custom_private_wallet(balance: WalletBalance, keys_pair: WalletKeys) -> PrivateWallet {
		let (wallet, _) = PrivateWallet::new();
		wallet.with_balance(balance).with_keys(keys_pair)
	}
	
	pub(crate) fn new_public_wallet(balance: WalletBalance) -> PublicWallet {
		new_private_wallet(balance).to_public()
	}
	
	pub(crate) fn custom_public_wallet(balance: WalletBalance, keys_pair: WalletKeys) -> PublicWallet {
		custom_private_wallet(balance, keys_pair).to_public()
	}
}
