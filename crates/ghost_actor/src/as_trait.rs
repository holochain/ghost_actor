use crate::*;

/// GhostActor trait allows constructs such as `Arc<dyn AsGhostActor>`.
pub trait AsGhostActor<T: 'static + Send>:
    'static + Send + Sync + Clone + PartialEq + Eq + std::hash::Hash
{
    /// Push state read/mutation logic onto actor queue for processing.
    fn invoke<R, E, F>(&self, invoke: F) -> GhostFuture<R, E>
    where
        R: 'static + Send,
        E: 'static + From<GhostError> + Send,
        F: FnOnce(Self, &mut T) -> Result<R, E> + 'static + Send;

    /// Returns `true` if the channel is still connected to the actor task.
    fn is_active(&self) -> bool;

    /// Close the channel to the actor task.
    /// This will result in the task being dropped once all pending invocations
    /// have been processed.
    fn shutdown(&self);
}
