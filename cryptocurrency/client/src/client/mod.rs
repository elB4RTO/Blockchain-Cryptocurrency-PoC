pub mod client;
mod service;


pub use client::*;


const ENV_VAR_SERVER_HOST : &str = "CRYPTOCURRENCY_SERVER_HOST";
const ENV_VAR_SERVER_PORT : &str = "CRYPTOCURRENCY_SERVER_PORT";

const ENV_VAR_CLIENT_HOST : &str = "CRYPTOCURRENCY_CLIENT_HOST";
const ENV_VAR_CLIENT_PORT : &str = "CRYPTOCURRENCY_CLIENT_PORT";


mod addresses {
	pub(super) const SERVER_HOST : &str = "127.0.0.1";
	pub(super) const SERVER_PORT : &str = "31337";

	pub(super) const HOST : &str = "127.0.0.1";
	pub(super) const PORT : &str = "0";
}


fn server_host() -> String {
    std::env::var(ENV_VAR_SERVER_HOST).unwrap_or_else(|_| addresses::SERVER_HOST.to_owned())
}

fn server_port() -> u16 {
    std::env::var(ENV_VAR_SERVER_PORT).unwrap_or_else(|_| addresses::SERVER_PORT.to_owned())
        .parse::<u16>().expect("Invalid server port")
}

fn server_address() -> String {
	format!("{}:{}", server_host(), server_port())
}


fn client_host() -> String {
    std::env::var(ENV_VAR_CLIENT_HOST).unwrap_or_else(|_| addresses::HOST.to_owned())
}

fn client_port() -> u16 {
    std::env::var(ENV_VAR_CLIENT_PORT).unwrap_or_else(|_| addresses::PORT.to_owned())
        .parse::<u16>().expect("Invalid client port")
}
