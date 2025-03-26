use mongodb::Database;

use crate::language_model::language_model_service::LanguageModelService;

#[derive(Clone)]
pub struct AppService {
    pub language_model_service: LanguageModelService,
}

impl AppService {
    pub fn new(_database: Database) -> Self {
        let language_model_service = LanguageModelService::new();

        Self {
            language_model_service,
        }
    }
}
#[derive(Clone)]
pub struct AppState {
    pub service: AppService,
    pub database: Database,
}
impl AppState {
    pub fn new(database: Database) -> Self {
        Self {
            service: AppService::new(database.clone()),
            database,
        }
    }
}
