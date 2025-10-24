
pub trait BytesConversion: Sized {
	type Error;
	
    fn as_boxed_bytes(&self) -> Result<Box<[u8]>, Self::Error>;
	
    fn as_actix_bytes(&self) -> Result<actix_web::web::Bytes, Self::Error> {
        self.as_boxed_bytes().map(|b| b.into())
    }
    
    fn from_raw_bytes(bytes: &[u8]) -> Result<Self, Self::Error>;
    
    fn from_actix_bytes(bytes: actix_web::web::Bytes) -> Result<Self, Self::Error> {
		Self::from_raw_bytes(&bytes)
	}
}


pub fn append_bytes(initial_bytes: actix_web::web::Bytes, following_bytes: actix_web::web::Bytes) -> actix_web::web::Bytes {
	initial_bytes.into_iter().chain(following_bytes.into_iter()).collect()
}


pub fn append_bytes_raw(bytes: &[u8], array: &mut [u8], array_index: &mut usize) -> Result<(), String> {
	if bytes.len() > array.len() - *array_index {
		return Err(String::from("utils::append_bytes_raw(): not enough space in the array"));
	}
	let final_array_index = (*array_index) + bytes.len();
	array[*array_index..final_array_index].copy_from_slice(bytes);
	*array_index = final_array_index;
	Ok(())
}


pub fn boxed_bytes(length: usize) -> Box<[u8]> {	
	let mut vec : Vec<u8> = Vec::with_capacity(length);
	vec.resize(length, 0);
	vec.into_boxed_slice()
}


#[cfg(test)]
mod test {
	use super::*;
	
	#[test]
    fn append_raw_bytes() {
		let expected_bytes: [u8; 8] = [1,1,1,1,2,2,2,2];
		let mut bytes: [u8; 8] = [0; 8];
		{
			let ref mut idx: usize = 0;
			append_bytes_raw(&[1,1,1,1], &mut bytes, idx).unwrap();
			append_bytes_raw(&[2,2,2,2], &mut bytes, idx).unwrap();
		}
		assert_eq!(bytes, expected_bytes);
	}
    
	#[test]
    fn append_actix_bytes() {
		let expected_bytes = actix_web::web::Bytes::from(vec![1,1,1,1,2,2,2,2]);
		let mut bytes = actix_web::web::Bytes::new();
		{
			bytes = append_bytes(bytes, actix_web::web::Bytes::from(vec![1,1,1,1]));
			bytes = append_bytes(bytes, actix_web::web::Bytes::from(vec![2,2,2,2]));
		}
		assert_eq!(bytes, expected_bytes);
	}
}

