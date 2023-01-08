use axum::{
    routing::{get, MethodRouter},
    Router,
};
use std::net::SocketAddr;
use tokio::time::{self, Duration};
use tokio::task;


mod arso;

fn root() -> Router {
    async fn handler() -> &'static str {
        "Try /metrics\r\n"
    }

    route("/", get(handler))
}

fn get_metrics() -> Router {
    async fn handler() -> String {
        match arso::arso_get_metrics().await {
            Ok(m) => m,
            Err(e) => e.to_string(),
        }
    }

    route("/metrics", get(handler))
}

fn route(path: &str, method_router: MethodRouter<()>) -> Router {
    Router::new().route(path, method_router)
}


#[tokio::main]
async fn main() {

    let _forever = task::spawn(async {
        let mut interval = time::interval(Duration::from_secs(20 * 60));

        loop {
            interval.tick().await;
            let _retrieve = arso::arso_retrieve().await;
        }
    });

    let app = Router::new()
        .merge(root())
        .merge(get_metrics());

    let addr = SocketAddr::from(([0, 0, 0, 0], 9336));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
