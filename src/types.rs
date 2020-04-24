/// Ghost error type.
#[derive(Debug, thiserror::Error)]
pub enum GhostError {
    /// Failed to send on channel.
    SendError(#[from] futures::channel::mpsc::SendError),

    /// Error sending response.
    ResponseError(#[from] futures::channel::oneshot::Canceled),

    /// Invalid custom type error.
    InvalidCustomType,

    /// Unspecified GhostActor error.
    Other(String),
}

impl std::fmt::Display for GhostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<&str> for GhostError {
    fn from(s: &str) -> Self {
        GhostError::Other(s.to_string())
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
    dyn FnOnce(I) -> ::must_future::MustBoxFuture<'static, std::result::Result<H, E>>
        + 'static
        + Send,
>;
