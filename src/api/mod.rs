mod chat;

use crate::types::AppState;
use axum::Router;
use std::sync::Arc;

pub(crate) fn create_router(app_context: Arc<AppState>) -> Router {
    Router::new().merge(chat::router(app_context))
}
