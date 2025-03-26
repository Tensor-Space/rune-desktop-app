pub mod app_module;
pub mod app_router;
pub mod health;
pub mod proxy_handler;
pub mod shared;

use axum::Router;
use dotenvy::dotenv;
use hyper_util::{client::legacy::connect::HttpConnector, rt::TokioExecutor};
use std::env;
use tracing::Level;
use tracing_subscriber::{fmt::format::FmtSpan, FmtSubscriber};

use crate::{app_module::GatekeeperService, app_router::proxy_router, proxy_handler::Client};

#[tokio::main]
async fn main() {
    dotenv().ok();
    let subscriber_builder = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_level(true)
        .with_line_number(true)
        .with_file(true)
        .with_thread_ids(true)
        .with_span_events(FmtSpan::CLOSE);

    if env::var("APP_ENVIRONMENT")
        .unwrap_or("prod".to_string())
        .parse::<String>()
        .unwrap()
        == "dev"
    {
        tracing::subscriber::set_global_default(
            subscriber_builder.compact().with_ansi(true).finish(),
        )
        .expect("setting dev subscriber failed");
    } else {
        tracing::subscriber::set_global_default(
            subscriber_builder.json().with_ansi(false).finish(),
        )
        .expect("setting prod subscriber failed");
    }
    let mut service = GatekeeperService::new();
    service.register_external_service(
        "http://engine-service".to_string(),
        "engine".to_string(),
        "v1".to_string(),
    );

    let client: Client =
        hyper_util::client::legacy::Client::<(), ()>::builder(TokioExecutor::new())
            .build(HttpConnector::new());

    let app = Router::new().merge(proxy_router(client, service));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
