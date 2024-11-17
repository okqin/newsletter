use clap::Parser;
use newsletter::{telemetry::setup_tracing, Args, HttpServer, Settings};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let conf = Settings::try_load(args.config).expect("Failed to read config.");
    setup_tracing(conf.logs.as_ref());
    HttpServer::try_new(&conf).await?.serve().await
}
