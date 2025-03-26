use anyhow::{anyhow, Result};
use std::collections::HashMap;

#[derive(Clone)]
pub struct ExternalService {
    base_url: String,
}

#[derive(Clone)]
pub struct GatekeeperService {
    external_services: HashMap<String, ExternalService>,
}
impl GatekeeperService {
    pub fn new() -> Self {
        Self {
            external_services: HashMap::new(),
        }
    }

    pub fn register_external_service(
        &mut self,
        base_url: String,
        service_name: String,
        version: String,
    ) {
        self.external_services.insert(
            format!("{}:{}", service_name, version),
            ExternalService { base_url },
        );
    }
    pub fn get_external_service_host_name(
        &self,
        service_name: String,
        version: String,
    ) -> Result<String> {
        if let Some(service) = self
            .external_services
            .get(&format!("{}:{}", service_name, version))
        {
            Ok(service.base_url.clone())
        } else {
            Err(anyhow!("no external service found with given params"))
        }
    }
}
