
use bytes_utils::BytesConversion;


pub(super) const SECONDS_BETWEEN_ECHOES: u64 = 15 * 60;
pub(super) const SECONDS_BETWEEN_PEERS_CHANGED: u64 = 1 * 60;


static PEERS: std::sync::LazyLock<std::sync::Mutex<peers::Peers>> = std::sync::LazyLock::new(|| std::sync::Mutex::new(peers::Peers::with_capacity(64)));

static ECHOES: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<peers::Peer, usize>>> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::with_capacity(64)));

static PEERS_CHANGED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);


pub(super) fn accomplish_request(request: actix_web::HttpRequest, request_bytes: actix_web::web::Bytes) -> actix_web::HttpResponse {
    if request_bytes == requests::server::CONNECT {
		add_peer(request)
	} else if request_bytes == requests::server::DISCONNECT {
		remove_peer(request)
	} else if request_bytes == requests::server::SEND_PEERS {
		send_peers(request)
	} else {
        actix_web::HttpResponse::BadRequest().finish()
    }
}

fn add_peer(request: actix_web::HttpRequest) -> actix_web::HttpResponse {
	match request.peer_addr() {
		Some(peer_addr) => {
			let peer = peers::Peer::from(peer_addr);
			let mut peers = PEERS.lock().unwrap();
			if peers.contains(&peer) {
				println!("service::add_peer(): peer already connected: {}", peer);
			} else {
				println!("service::add_peer(): new peer connected: {}", peer);
				peers.insert(peer.clone());
				ECHOES.lock().unwrap().insert(peer, 0);
				PEERS_CHANGED.store(true, std::sync::atomic::Ordering::SeqCst);
			}
			actix_web::HttpResponse::Ok().finish()
		},
		None => {
			eprintln!("service::add_peer(): failed to retrieve peer address from request");
			actix_web::HttpResponse::InternalServerError().finish()
		},
	}
}

fn remove_peer(request: actix_web::HttpRequest) -> actix_web::HttpResponse {
	match request.peer_addr() {
		Some(peer_addr) => {
			let peer = peers::Peer::from(peer_addr);
			let mut peers = PEERS.lock().unwrap();
			if peers.contains(&peer) {
				println!("service::remove_peer(): peer disconnected: {}", peer);
				peers.remove(&peer);
				ECHOES.lock().unwrap().remove(&peer);
				PEERS_CHANGED.store(true, std::sync::atomic::Ordering::SeqCst);
			} else {
				println!("service::remove_peer(): peer was not connected: {}", peer);
			}
			actix_web::HttpResponse::Ok().finish()
		},
		None => {
			eprintln!("service::remove_peer(): failed to retrieve peer address from request");
			actix_web::HttpResponse::InternalServerError().finish()
		},
	}
}

fn send_peers(request: actix_web::HttpRequest) -> actix_web::HttpResponse {
	match request.peer_addr() {
		Some(peer_addr) => {
			let peer = peers::Peer::from(peer_addr);
			let peers = PEERS.lock().unwrap();
			let request_body = if peers.contains(&peer) {
				println!("service::send_peers(): peer requested peers: {}", peer);
				peers.as_actix_bytes()
					.unwrap_or_else(|e| {
						eprintln!("service::send_peers(): failed to convert peers to bytes: {}", e);
						actix_web::web::Bytes::new()
					})
			} else {
				println!("service::send_peers(): peer not in peers list: {}", peer);
				actix_web::web::Bytes::new()
			};
			actix_web::HttpResponse::Ok().body(request_body)
		},
		None => {
			eprintln!("service::send_peers(): failed to retrieve peer address from request");
			actix_web::HttpResponse::InternalServerError().finish()
		},
	}
}


pub(super) async fn notify_peers_changed() {
	if !PEERS_CHANGED.swap(false, std::sync::atomic::Ordering::SeqCst) {
		return;
	}
	let peers = PEERS.lock().unwrap().clone();
	for peer in peers {
		let client = reqwest::ClientBuilder::new()
			.default_headers(reqwest::header::HeaderMap::new())
			.retry(reqwest::retry::never())
			.timeout(std::time::Duration::from_secs(5))
			.build();
		if let Err(e) = client {
			eprintln!("service::notify_peers_changed(): failed to create client: {}", e);
			continue;
		}
		let response = client.unwrap().get(format!("{}", peer))
			.body(requests::server::PEERS_CHANGED)
			.send()
			.await;
		if let Err(e) = response {
			eprintln!("service::notify_peers_changed(): failed to send request: {}", e);
			continue;
		}
	}
}


pub(super) async fn send_echoes() {
	let mut peers_to_remove: Vec<peers::Peer> = Vec::new();
	let original_peers = PEERS.lock().unwrap().clone();
	for peer in original_peers {
		let client = reqwest::ClientBuilder::new()
			.default_headers(reqwest::header::HeaderMap::new())
			.retry(reqwest::retry::never())
			.timeout(std::time::Duration::from_secs(5))
			.build();
		if let Err(e) = client {
			eprintln!("service::send_ehcoes(): failed to create client: {}", e);
			continue;
		}
		let response = client.unwrap().get(format!("{}", peer))
			.body(requests::server::ALIVE_ECHO)
			.send()
			.await;
		if let Err(e) = response {
			eprintln!("service::send_ehcoes(): failed to send request: {}", e);
			continue;
		}
		let response_body = response.unwrap().bytes().await;
		if let Err(e) = response_body {
			eprintln!("service::send_ehcoes(): failed to retrieve response body: {}", e);
			continue;
		}
		if let Some(echo) = ECHOES.lock().unwrap().get_mut(&peer) {
			if response_body.unwrap() == requests::server::ALIVE_ECHO {
				*echo = 0;
			} else {
				*echo += 1;
				if *echo > 3 {
					peers_to_remove.push(peer.clone());
				}
			}
		} else {
			eprintln!("service::send_ehcoes(): peer not found in echoes: {}", peer);
		}
	}
	let mut peers = PEERS.lock().unwrap();
	let mut echoes = ECHOES.lock().unwrap();
	for peer in peers_to_remove.iter() {
		let _ = peers.remove(peer);
		let _ = echoes.remove(peer);
	}
}

