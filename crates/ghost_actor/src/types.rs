/// Ghost error type.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GhostError {
    /// Failed to send on channel.
    #[error(transparent)]
    SendError(#[from] futures::channel::mpsc::SendError),

    /// Error sending response.
    #[error(transparent)]
    ResponseError(#[from] futures::channel::oneshot::Canceled),

    /// Invalid custom type error.
    #[error("InvalidCustomType")]
    InvalidCustomType,

    /// Unspecified GhostActor error.
    #[error(transparent)]
    Other(Box<dyn std::error::Error + Send + Sync>),
}

impl GhostError {
    /// Build an "Other" type GhostError.
    pub fn other(
        e: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        GhostError::Other(e.into())
    }
}

impl From<String> for GhostError {
    fn from(s: String) -> Self {
        #[derive(Debug, thiserror::Error)]
        struct OtherError(String);
        impl std::fmt::Display for OtherError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        GhostError::other(OtherError(s))
    }
}

impl From<&str> for GhostError {
    fn from(s: &str) -> Self {
        s.to_string().into()
    }
}

impl From<GhostError> for () {
    fn from(_: GhostError) {}
}

/// Ghost Result Type.
pub type GhostResult<T> = Result<T, GhostError>;

/// This future represents a spawned GhostActor task, you must await
/// or spawn this task into an executor for the actor to function.
pub type GhostActorDriver = ::must_future::MustBoxFuture<'static, ()>;

/// This is the factory callback signature for spawning new actor tasks.
pub type GhostActorSpawn<I, H, E> = Box<
    dyn FnOnce(
            I,
        ) -> ::must_future::MustBoxFuture<
            'static,
            std::result::Result<H, E>,
        >
        + 'static
        + Send,
>;
