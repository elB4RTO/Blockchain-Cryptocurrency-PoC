use ed25519_dalek::{SigningKey, Signer, Signature, VerifyingKey, Verifier};
use rand::rngs::OsRng;


pub(crate) fn generate_keys_pair() -> SigningKey {
	let mut csprng = OsRng;
	SigningKey::generate(&mut csprng)
}


pub(crate) fn sign(keys_pair: &SigningKey, message: &[u8]) -> Signature {
	keys_pair.sign(message)
}

pub(crate) fn verify_private(keys_pair: &SigningKey, signature: &Signature, message: &[u8]) -> bool {
	keys_pair.verify(message, signature).is_ok()
}

pub(crate) fn verify_public(key: &VerifyingKey, signature: &Signature, message: &[u8]) -> bool {
	key.verify(message, signature).is_ok()
}


#[cfg(test)]
mod test {
	use super::*;
	
	#[test]
	fn generate_keys() {
		let _ = generate_keys_pair();
	}
	
	#[test]
	fn sign_message() {
		let keys = generate_keys_pair();
		let message : [u8; 16] = [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F];
		
		let _ = sign(&keys, &message);
	}
	
	#[test]
	fn verify_signed_message_private() {
		let keys = generate_keys_pair();
		let message : [u8; 16] = [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F];
		let signature = sign(&keys, &message);
		
		assert!(verify_private(&keys, &signature, &message));
	}
	
	#[test]
	fn verify_signed_message_public() {
		let keys = generate_keys_pair();
		let message : [u8; 16] = [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F];
		let signature = sign(&keys, &message);
		
		assert!(verify_public(&keys.verifying_key(), &signature, &message));
	}
}
