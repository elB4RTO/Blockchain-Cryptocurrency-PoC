use crate::server::{HOST, PORT};
use crate::server::service;


const ENV_VAR_SERVER_HOST : &str = "CRYPTOCURRENCY_SERVER_HOST";
const ENV_VAR_SERVER_PORT : &str = "CRYPTOCURRENCY_SERVER_PORT";


fn server_host() -> String {
    std::env::var(ENV_VAR_SERVER_HOST).unwrap_or_else(|_| HOST.to_owned())
}

fn server_port() -> u16 {
    std::env::var(ENV_VAR_SERVER_PORT).unwrap_or_else(|_| PORT.to_owned())
        .parse::<u16>().expect("Invalid server port")
}


pub async fn run() -> std::io::Result<tokio::task::JoinSet<()>> {
	actix_web::HttpServer::new(|| actix_web::App::new().service(handler))
		.bind((server_host(), server_port()))?
		.run().await?;
	let mut join_set = tokio::task::JoinSet::new();
	join_set.spawn(async move {
		loop {
			service::send_echoes().await;
			tokio::time::sleep(tokio::time::Duration::from_secs(service::SECONDS_BETWEEN_ECHOES)).await;
		}
	});
	join_set.spawn(async move {
		loop {
			service::notify_peers_changed().await;
			tokio::time::sleep(tokio::time::Duration::from_secs(service::SECONDS_BETWEEN_PEERS_CHANGED)).await;
		}
	});
	Ok(join_set)
}

#[actix_web::post("/")]
async fn handler(request: actix_web::HttpRequest, bytes: actix_web::web::Bytes) -> actix_web::HttpResponse {
    service::accomplish_request(request, bytes)
}
