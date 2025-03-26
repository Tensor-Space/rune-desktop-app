use axum::{middleware, routing::any, Extension, Router};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    app_module::GatekeeperService,
    health::health_controller::health,
    proxy_handler::{Client, GatekeeperProxyHandler},
    shared::auth_middleware::authorize,
};

pub fn proxy_router(client: Client, service: GatekeeperService) -> Router {
    Router::new()
        // Protected Routes go here
        .route(
            "/:service/:version/protext-route-here",
            any(GatekeeperProxyHandler::handle),
        )
        .route_layer(middleware::from_fn(authorize))
        // Public Routes go here
        .route(
            "/:service/:version/*path",
            any(GatekeeperProxyHandler::handle),
        )
        .route(
            "/:service/:version/health",
            any(GatekeeperProxyHandler::handle),
        )
        .route("/gatekeeper/v1/health", any(health))
        .with_state(client)
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any),
        )
        .layer(Extension(service))
}
