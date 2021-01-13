use std::sync::Arc;

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

impl From<String> for GhostError {
    fn from(s: String) -> Self {
        #[derive(Debug)]
        struct OtherError(String);
        impl std::fmt::Display for OtherError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
        impl std::error::Error for OtherError {}

        GhostError::other(OtherError(s))
    }
}

impl From<&str> for GhostError {
    fn from(s: &str) -> Self {
        s.to_string().into()
    }
}

impl From<GhostError> for () {
    fn from(_: GhostError) -> Self {}
}
