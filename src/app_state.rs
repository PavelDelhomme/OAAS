use std::sync::Arc;

use crate::models_catalog::ModelsCatalogRoot;
use crate::proxy::ProxyState;
use crate::serve_dashboard::OaasStatus;

#[derive(Clone)]
pub struct AppState {
    pub proxy: ProxyState,
    pub catalog: Arc<ModelsCatalogRoot>,
    pub status: Arc<OaasStatus>,
    /// PID `llama-server` pour `/oaas/system.json` (Unix).
    pub llama_pid: Option<u32>,
}
