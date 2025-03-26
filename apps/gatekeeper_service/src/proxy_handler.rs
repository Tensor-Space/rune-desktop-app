use axum::{
    body::Body,
    extract::{Request, State},
    http::uri::Uri,
    response::{IntoResponse, Response},
    Extension,
};
use hyper::StatusCode;
use hyper_util::client::legacy::connect::HttpConnector;
use tracing::info;

use crate::app_module::GatekeeperService;

pub type Client = hyper_util::client::legacy::Client<HttpConnector, Body>;

#[derive(Clone)]
pub struct GatekeeperProxyHandler {}
impl GatekeeperProxyHandler {
    pub async fn handle(
        State(client): State<Client>,
        Extension(service): Extension<GatekeeperService>,
        mut req: Request,
    ) -> Result<Response, StatusCode> {
        info!("triggered proxy handler, routing request to respective service");
        let query = req
            .uri()
            .query()
            .map(|q| format!("?{}", q))
            .unwrap_or("".to_string());

        let mut path_parts: Vec<&str> = req.uri().path().split('/').collect();
        let service_name = path_parts[1];
        let version = path_parts[2];
        let hostname =
            service.get_external_service_host_name(service_name.to_string(), version.to_string());
        let path_parts = path_parts.split_off(3);
        let path = path_parts.join("/");

        match hostname {
            Ok(hostname) => {
                let uri = format!("{}/{}/{}{}", hostname, version, path, query);
                *req.uri_mut() = Uri::try_from(uri).unwrap();

                Ok(client
                    .request(req)
                    .await
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
                    .into_response())
            }
            Err(_err) => Err(StatusCode::NOT_FOUND),
        }
    }
}
