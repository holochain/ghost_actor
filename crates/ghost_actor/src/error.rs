use crate::*;

/// Generic GhostActor Error Type
#[derive(Debug, Clone)]
pub struct GhostError(pub Arc<dyn std::error::Error + Send + Sync>);

impl std::fmt::Display for GhostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for GhostError {}

impl GhostError {
    /// Convert a std Error into a GhostError
    pub fn other<E: 'static + std::error::Error + Send + Sync>(e: E) -> Self {
        Self(Arc::new(e))
    }
}

impl From<GhostError> for () {
    fn from(_: GhostError) -> Self {}
}
