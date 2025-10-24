use crate::client::*;
use crate::client::service;


pub async fn run() -> std::io::Result<()> {
    actix_web::HttpServer::new(|| actix_web::App::new().service(handler))
		.bind((client_host(), client_port()))?
		.run().await
}

#[actix_web::post("/")]
async fn handler(bytes: actix_web::web::Bytes) -> actix_web::HttpResponse {
    service::accomplish_request(bytes)
}


pub async fn notify_connection() -> bool {
	service::notify_connection().await
}

pub async fn notify_disconnection() -> bool {
	service::notify_disconnection().await
}

pub async fn request_peers() -> bool {
	service::request_peers().await
}


pub async fn request_blockchain() -> bool {
	service::request_blockchain().await
}

pub async fn notify_block_mined(mined_block: blockchain::block::Block) -> bool {
	service::notify_block_mined(mined_block).await
}
