use newsletter::{configuration, startup};

#[tokio::main]
async fn main() {
    let cfg = configuration::get_configuration().expect("Failed to read config");
    let addr = format!("0.0.0.0:{}", cfg.server_port);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind address");
    startup::run(listener).await
}
