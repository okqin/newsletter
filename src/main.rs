use newsletter::{telemetry::setup_tracing, HttpServer, Settings};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let conf = Settings::try_load().expect("Failed to read config");
    setup_tracing(conf.logs.as_ref());
    HttpServer::try_new(&conf).await?.serve().await
}
