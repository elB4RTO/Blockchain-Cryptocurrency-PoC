use crate::client::server_address;

use bytes_utils::BytesConversion;


static PEERS: std::sync::LazyLock<std::sync::Mutex<peers::Peers>> = std::sync::LazyLock::new(|| std::sync::Mutex::new(peers::Peers::with_capacity(64)));


pub(super) fn accomplish_request(request_bytes: actix_web::web::Bytes) -> actix_web::HttpResponse {
    if request_bytes == requests::server::ALIVE_ECHO {
		send_echo()
    } else if request_bytes == requests::server::PEERS_CHANGED {
		request_peers();
		actix_web::HttpResponse::Ok().finish()
    } else if request_bytes == requests::clients::SEND_BLOCKCHAIN {
		send_blockchain()
	} else if request_bytes[..64] == requests::clients::VALIDATE_BLOCK {
		validate_block(request_bytes.slice(64..))
	} else if request_bytes[..64] == requests::clients::ADD_BLOCK {
		add_block(request_bytes.slice(64..))
	} else {
        actix_web::HttpResponse::BadRequest().finish()
    }
}

fn send_echo() -> actix_web::HttpResponse {
	actix_web::HttpResponse::Ok().body(requests::server::ALIVE_ECHO)
}

fn send_blockchain() -> actix_web::HttpResponse {
	match blockchain::as_actix_bytes() {
		Ok(blockchain_bytes) => actix_web::HttpResponse::Ok().body(blockchain_bytes),
		Err(e) => {
			eprintln!("service::send_blockchain(): failed to convert to bytes: {e}");
			actix_web::HttpResponse::InternalServerError().finish()
		},
	}
}

fn validate_block(block_bytes: actix_web::web::Bytes) -> actix_web::HttpResponse {
	match blockchain::block::Block::from_actix_bytes(block_bytes) {
		Ok(block) => {
			let request_response = match blockchain::validate_last_block(&block) {
				Ok(_) => requests::clients::responses::BLOCK_VALID,
				Err(_) => requests::clients::responses::BLOCK_INVALID,
			};
			actix_web::HttpResponse::Ok().body(request_response)
		},
		Err(e) => {
			eprintln!("service::validate_block(): failed to retrieve block from bytes: {e}");
			actix_web::HttpResponse::InternalServerError().finish()
		},
	}
}

fn add_block(block_bytes: actix_web::web::Bytes) -> actix_web::HttpResponse {
	match blockchain::block::Block::from_actix_bytes(block_bytes) {
		Ok(block) => {
			blockchain::add_block(block);
			actix_web::HttpResponse::Ok().finish()
		},
		Err(e) => {
			eprintln!("service::add_block(): failed to retrieve block from bytes: {e}");
			actix_web::HttpResponse::InternalServerError().finish()
		},
	}
}


pub(super) async fn notify_connection() -> bool {
	let client = reqwest::ClientBuilder::new()
		.default_headers(reqwest::header::HeaderMap::new())
		//.retry(reqwest::retry::never())
		.timeout(std::time::Duration::from_secs(60))
		.build();
	if let Err(e) = client {
		eprintln!("service::notify_connection(): failed to create client: {}", e);
		return false;
	}
	let response = client.unwrap().get(server_address())
		.body(requests::server::CONNECT)
		.send()
		.await;
	if let Err(e) = response {
		eprintln!("service::notify_connection(): failed to send request: {}", e);
		return false;
	}
	let response_status = response.unwrap().status();
	if !response_status.is_success() {
		eprintln!("service::notify_connection(): response status is not success: {}", response_status.as_u16());
		return false;
	}
	true
}

pub(super) async fn notify_disconnection() -> bool {
	let client = reqwest::ClientBuilder::new()
		.default_headers(reqwest::header::HeaderMap::new())
		//.retry(reqwest::retry::never())
		.timeout(std::time::Duration::from_secs(60))
		.build();
	if let Err(e) = client {
		eprintln!("service::notify_disconnection(): failed to create client: {}", e);
		return false;
	}
	let response = client.unwrap().get(server_address())
		.body(requests::server::DISCONNECT)
		.send()
		.await;
	if let Err(e) = response {
		eprintln!("service::notify_disconnection(): failed to send request: {}", e);
		return false;
	}
	let response_status = response.unwrap().status();
	if !response_status.is_success() {
		eprintln!("service::notify_disconnection(): response status is not success: {}", response_status.as_u16());
		return false;
	}
	true
}

pub(super) async fn request_peers() -> bool {
	let client = reqwest::ClientBuilder::new()
		.default_headers(reqwest::header::HeaderMap::new())
		//.retry(reqwest::retry::never())
		.timeout(std::time::Duration::from_secs(60))
		.build();
	if let Err(e) = client {
		eprintln!("service::request_peers(): failed to create client: {}", e);
		return false;
	}
	let response = client.unwrap().get(server_address())
		.body(requests::server::SEND_PEERS)
		.send()
		.await;
	if let Err(e) = response {
		eprintln!("service::request_peers(): failed to send request: {}", e);
		return false;
	}
	let response = response.unwrap();
	let response_status = response.status();
	if !response_status.is_success() {
		eprintln!("service::request_peers(): response status is not success: {}", response_status.as_u16());
		return false;
	}
	let response_body = response.bytes().await;
	if let Err(e) = response_body {
		eprintln!("service::request_peers(): failed to retrieve response body: {}", e);
		return false;
	}
	let new_peers = peers::Peers::from_actix_bytes(response_body.unwrap());
	if let Err(e) = new_peers {
		eprintln!("service::request_peers(): failed to retrieve peers from bytes: {}", e);
		return false;
	}
	*PEERS.lock().unwrap() = new_peers.unwrap();
	true
}

pub(super) async fn request_blockchain() -> bool {
	let request_peer_validation = async |peer_address: String, request_body: actix_web::web::Bytes| -> bool {
		let client = reqwest::ClientBuilder::new()
			.default_headers(reqwest::header::HeaderMap::new())
			//.retry(reqwest::retry::never())
			.timeout(std::time::Duration::from_secs(30))
			.build();
		if let Err(e) = client {
			eprintln!("service::request_blockchain()::request_peer_validation(): failed to create client: {}", e);
			return false;
		}
		let response = client.unwrap().get(peer_address)
			.body(request_body)
			.send()
			.await;
		if let Err(e) = response {
			eprintln!("service::request_blockchain()::request_peer_validation(): failed to send request: {}", e);
			return false;
		}
		let response = response.unwrap();
		let response_status = response.status();
		if !response_status.is_success() {
			eprintln!("service::request_blockchain()::request_peer_validation(): response status is not success: {}", response_status.as_u16());
			return false;
		}
		let response_body = response.bytes().await;
		if let Err(e) = response_body {
			eprintln!("service::request_blockchain()::request_peer_validation(): failed to retrieve response body: {}", e);
			return false;
		}
		let response_body = response_body.unwrap();
		if response_body == requests::clients::responses::BLOCK_VALID {
			return true;
		} else if response_body == requests::clients::responses::BLOCK_INVALID {
			return false;
		} else {
			eprintln!("service::request_blockchain()::request_peer_validation(): unexpected response: {:?}", response_body.to_vec());
			return false;
		}
	};
	let request_peer_blockchain = async |peer_address: String| -> Option<blockchain::blockchain::Blockchain> {
		let client = reqwest::ClientBuilder::new()
			.default_headers(reqwest::header::HeaderMap::new())
			//.retry(reqwest::retry::never())
			.timeout(std::time::Duration::from_secs(30))
			.build();
		if let Err(e) = client {
			eprintln!("service::request_blockchain()::request_peer_blockchain(): failed to create client: {}", e);
			return None;
		}
		let response = client.unwrap().get(peer_address)
			.body(requests::clients::SEND_BLOCKCHAIN)
			.send()
			.await;
		if let Err(e) = response {
			eprintln!("service::request_blockchain()::request_peer_blockchain(): failed to send request: {}", e);
			return None;
		}
		let response = response.unwrap();
		let response_status = response.status();
		if !response_status.is_success() {
			eprintln!("service::request_blockchain()::request_peer_blockchain(): response status is not success: {}", response_status.as_u16());
			return None;
		}
		let response_body = response.bytes().await;
		if let Err(e) = response_body {
			eprintln!("service::request_blockchain()::request_peer_blockchain(): failed to retrieve response body: {}", e);
			return None;
		}
		let new_blockchain = blockchain::blockchain::Blockchain::from_actix_bytes(response_body.unwrap());
		if let Err(e) = new_blockchain {
			eprintln!("service::request_blockchain()::request_peer_blockchain(): failed to retrieve blockchain from bytes: {}", e);
			return None;
		}
		new_blockchain.ok()
	};
	let peers = PEERS.lock().unwrap().clone();
	for peer in peers.iter() {
		let sender_peer_address = format!("{}", peer);
		if let Some(received_blockchain) = request_peer_blockchain(sender_peer_address).await {
			if blockchain::validate_blockchain(&received_blockchain).is_err() {
				continue;
			}
			let block_bytes = received_blockchain.last_block().as_boxed_bytes();
			if let Err(e) = block_bytes {
				eprintln!("service::request_blockchain(): failed to convert block to bytes: {}", e);
				continue;
			}
			let mut request_body = actix_web::web::BytesMut::from(requests::clients::VALIDATE_BLOCK);
			request_body.extend_from_slice(&block_bytes.unwrap());
			let request_body = actix_web::web::Bytes::from(request_body);
			let mut valid_blockchain_responses: usize = 0;
			let mut invalid_blockchain_responses: usize = 0;
			for peer in peers.iter() {
				let validator_peer_address = format!("{}", peer);
				if request_peer_validation(validator_peer_address, request_body.clone()).await {
					valid_blockchain_responses += 1;
				} else {
					invalid_blockchain_responses += 1;
				}				
			}
			if !has_consensum(valid_blockchain_responses, invalid_blockchain_responses) {
				continue;
			}
			if let Err(e) = blockchain::update(received_blockchain) {
				eprintln!("service::request_blockchain(): failed to update blockchain: {}", e);
				continue;
			}
			return true;
		}
	}
	false
}

pub(super) async fn notify_block_mined(mined_block: blockchain::block::Block) -> bool {
	let notify_peer = async |peer_address: String, request_body: actix_web::web::Bytes| -> Option<bool> {
		let client = reqwest::ClientBuilder::new()
			.default_headers(reqwest::header::HeaderMap::new())
			//.retry(reqwest::retry::never())
			.timeout(std::time::Duration::from_secs(30))
			.build();
		if let Err(e) = client {
			eprintln!("service::notify_block_mined(): failed to create client: {}", e);
			return None;
		}
		let response = client.unwrap().get(peer_address)
			.body(request_body)
			.send()
			.await;
		if let Err(e) = response {
			eprintln!("service::notify_block_mined(): failed to send request: {}", e);
			return None;
		}
		let response = response.unwrap();
		let response_status = response.status();
		if !response_status.is_success() {
			eprintln!("service::notify_block_mined(): response status is not success: {}", response_status.as_u16());
			return None;
		}
		let response_body = response.bytes().await;
		if let Err(e) = response_body {
			eprintln!("service::notify_block_mined(): failed to retrieve response body: {}", e);
			return None;
		}
		let response_body = response_body.unwrap();
		if response_body == requests::clients::responses::BLOCK_VALID {
			return Some(true);
		} else if response_body == requests::clients::responses::BLOCK_INVALID {
			return Some(false);
		} else {
			eprintln!("service::notify_block_mined(): unexpected response: {:?}", response_body.to_vec());
			return None;
		}
	};
	let block_bytes = mined_block.as_boxed_bytes();
	if let Err(e) = block_bytes {
		eprintln!("service::notify_block_mined(): failed to convert block to bytes: {}", e);
		return false;
	}
	let mut request_body = actix_web::web::BytesMut::from(requests::clients::ADD_BLOCK);
	request_body.extend_from_slice(&block_bytes.unwrap());
	let request_body = actix_web::web::Bytes::from(request_body);
	let mut valid_block_responses: usize = 0;
	let mut invalid_block_responses: usize = 0;
	for peer in PEERS.lock().unwrap().iter().cloned() {
		let peer_address = format!("{}", peer);
		match notify_peer(peer_address, request_body.clone()).await {
			Some(true) => valid_block_responses += 1,
			Some(false) => invalid_block_responses += 1,
			_ => (),
		}
	}
	has_consensum(valid_block_responses, invalid_block_responses)
}


fn number_of_peers() -> usize {
	PEERS.lock().unwrap().len()
}

fn half_of_peers() -> usize {
	number_of_peers() / 2
}

fn has_consensum(valid_responses: usize, invalid_responses: usize) -> bool {
	valid_responses > invalid_responses && valid_responses > half_of_peers()
}

