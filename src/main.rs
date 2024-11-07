use newsletter::{get_configuration, HttpServer};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let conf = get_configuration().expect("Failed to read config");
    HttpServer::new(conf).await?.serve().await
}
