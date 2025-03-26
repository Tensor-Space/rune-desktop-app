use axum::{routing::get, Router};

use crate::{
    health::health_controller, language_model::language_model_controller::language_model_router,
};

pub fn application_router() -> Router {
    Router::new()
        .route("/v1/health", get(health_controller::health))
        .nest("/v1/language-model", language_model_router())
}
