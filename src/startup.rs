use crate::routes::router;
use tokio::net::TcpListener;

pub async fn run(listener: TcpListener) {
    let router = router();
    axum::serve(listener, router)
        .await
        .expect("Failed to start server")
}
