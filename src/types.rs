/// GhostActor error type.
#[derive(Debug, thiserror::Error)]
pub enum GhostActorError {
    /// Failed to send on channel
    SendError(#[from] futures::channel::mpsc::SendError),

    /// Error sending response
    ResponseError(#[from] futures::channel::oneshot::Canceled),

    /// unspecified ghost actor error
    Other(String),
}

impl std::fmt::Display for GhostActorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<&str> for GhostActorError {
    fn from(s: &str) -> Self {
        GhostActorError::Other(s.to_string())
    }
}

impl From<GhostActorError> for () {
    fn from(_: GhostActorError) {}
}

/// Trait for specifying Custom and Internal request types for GhostActors.
/// The only default impl is `()`, this may require you to create newtypes
/// if you wish to use basic types for messaging.
/// Pro tip: you can set the ResponseType to the same struct/enum
/// as your GhostRequestType impl.
pub trait GhostRequestType: 'static + Send + Clone {
    /// When you make a request of this type, what will be the response type?
    type ResponseType: 'static + Send + Clone;
}

impl GhostRequestType for () {
    type ResponseType = ();
}

/// This future represents a spawned GhostActor task, you must await
/// or spawn this task into an executor for the actor to function.
pub type GhostActorDriver = ::futures::future::BoxFuture<'static, ()>;
