use std::sync::Arc;

use sqlx::SqlitePool;

use crate::config::Config;
use crate::ln::LnBackend;
use crate::ws::NotificationHub;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub ln: Arc<dyn LnBackend>,
    pub hub: NotificationHub,
    pub config: Arc<Config>,
}
