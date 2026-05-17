mod chat;

use crate::types::AppContext;
use axum::Router;
use std::sync::Arc;

pub(crate) fn create_router(app_context: Arc<AppContext>) -> Router {
    Router::new().merge(chat::router(app_context))
}
