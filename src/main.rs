use axum::{
    routing::{get, MethodRouter},
    Router,
};
use config::{Config, ConfigError};
use serde::Deserialize;
use std::net::SocketAddr;
use tokio::task;
use tokio::time::{self, Duration};

mod arso;

#[derive(Debug, Deserialize)]
struct Settings {
    cities: Vec<String>,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        Config::builder()
            .add_source(config::File::with_name("settings"))
            .build()?
            .try_deserialize()
    }
}

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
    let settings = Settings::new().unwrap();

    let _forever = task::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(10 * 60));

        loop {
            interval.tick().await;
            let status = arso::arso_retrieve(&settings.cities).await;
            if let Err(err) = status {
                println!("{err}");
            }
        }
    });

    let app = Router::new().merge(root()).merge(get_metrics());

    let addr = SocketAddr::from(([0, 0, 0, 0], 9336));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
