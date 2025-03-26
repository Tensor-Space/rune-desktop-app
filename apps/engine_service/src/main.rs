use std::{env, error::Error, time::Duration};

use axum::{error_handling::HandleErrorLayer, http::StatusCode, BoxError, Extension, Router};
use dotenvy::dotenv;
use engine_service::{app_module::AppState, app_router::application_router};
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{fmt::format::FmtSpan, FmtSubscriber};

#[tokio::main]
async fn main() {
    dotenv().ok();
    let subscriber_builder = FmtSubscriber::builder()
        .with_level(true)
        .with_span_events(FmtSpan::CLOSE);

    if env::var("APP_ENVIRONMENT")
        .unwrap_or("dev".to_string())
        .parse::<String>()
        .unwrap()
        == "dev"
    {
        tracing::subscriber::set_global_default(
            subscriber_builder
                .compact()
                .pretty()
                .with_ansi(true)
                .finish(),
        )
        .expect("setting dev subscriber failed");
    } else {
        tracing::subscriber::set_global_default(
            subscriber_builder.json().with_ansi(false).finish(),
        )
        .expect("setting prod subscriber failed");
    }
    let database = setup_mongodb().await.unwrap();

    let state = AppState::new(database);

    let app = Router::new().merge(application_router()).layer(
        ServiceBuilder::new()
            .layer(HandleErrorLayer::new(|error: BoxError| async move {
                if error.is::<tower::timeout::error::Elapsed>() {
                    Ok(StatusCode::REQUEST_TIMEOUT)
                } else {
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Unhandled internal error: {}", error),
                    ))
                }
            }))
            .timeout(Duration::from_secs(10))
            .layer(TraceLayer::new_for_http())
            .layer(Extension(state))
            .layer(
                CorsLayer::new()
                    .allow_origin(tower_http::cors::Any)
                    .allow_methods(tower_http::cors::Any)
                    .allow_headers(tower_http::cors::Any),
            )
            .into_inner(),
    );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000")
        .await
        .expect("unable to create listner");

    tracing::info!("Server started, listening on port 8000");
    axum::serve(listener, app)
        .await
        .expect("unable to start srver");
}

async fn setup_mongodb() -> Result<mongodb::Database, Box<dyn Error>> {
    let uri = env::var("DATABASE_URI")?;
    let db_name = env::var("DATABASE_NAME").unwrap_or("RuneAI".to_string());

    let client = mongodb::Client::with_uri_str(&uri).await?;
    Ok(client.database(&db_name))
}
