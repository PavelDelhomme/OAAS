use std::sync::Arc;

use crate::serve_dashboard::OaasStatus;
use crate::models_catalog::ModelsCatalogRoot;
use crate::proxy::ProxyState;

#[derive(Clone)]
pub struct AppState {
    pub proxy: ProxyState,
    pub catalog: Arc<ModelsCatalogRoot>,
    pub status: Arc<OaasStatus>,
}
