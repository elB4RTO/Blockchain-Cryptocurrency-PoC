mod client;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	blockchain::init();
    client::run().await?;
    Ok(())
}
