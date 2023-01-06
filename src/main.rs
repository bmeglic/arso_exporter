use axum::{
    routing::{get, MethodRouter},
    Router,
};
use std::net::SocketAddr;

mod arso;

#[derive(Debug)]
enum Error {
    Timeout,
}

fn root() -> Router {
    async fn handler() -> &'static str {
        "Try /metrics\r\n"
    }

    route("/", get(handler))
}

fn get_metrics() -> Router {
    async fn handler() -> String {
        arso::arso_get_metrics().unwrap_or("Error\r\n".to_string())
    }

    route("/metrics", get(handler))
}

fn route(path: &str, method_router: MethodRouter<()>) -> Router {
    Router::new().route(path, method_router)
}

#[tokio::main]
async fn main() {
    let app = Router::new().merge(root()).merge(get_metrics());

    let addr = SocketAddr::from(([127, 0, 0, 1], 9336));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
