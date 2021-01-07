#![forbid(unsafe_code)]
#![forbid(warnings)]
#![forbid(missing_docs)]
//! GhostActor makes it simple, ergonomic, and idiomatic to implement
//! async / concurrent code using an Actor model.

use std::sync::Arc;

/// Generic GhostActor Error Type
#[derive(Debug, Clone)]
pub struct GhostError(Arc<dyn std::error::Error + Send + Sync>);

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

/// Driver future representing an actor task.
/// Please spawn this into whatever executor framework you are using.
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct ActorDriver(futures::future::BoxFuture<'static, ()>);

impl std::future::Future for ActorDriver {
    type Output = ();

    #[inline]
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context,
    ) -> std::task::Poll<Self::Output> {
        std::future::Future::poll(self.0.as_mut(), cx)
    }
}

/// Result future for GhostActor#invoke().
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct InvokeResult<T, E>(
    futures::future::BoxFuture<'static, Result<T, E>>,
)
where
    E: 'static + From<GhostError> + Send;

impl<T, E> InvokeResult<T, E>
where
    E: 'static + From<GhostError> + Send,
{
    /// Wrap another compatible future in an InvokeResult.
    #[inline]
    pub fn new<F>(f: F) -> Self
    where
        F: 'static + std::future::Future<Output = Result<T, E>> + Send,
    {
        Self(futures::future::FutureExt::boxed(f))
    }
}

impl<T, E> std::future::Future for InvokeResult<T, E>
where
    E: 'static + From<GhostError> + Send,
{
    type Output = Result<T, E>;

    #[inline]
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context,
    ) -> std::task::Poll<Self::Output> {
        std::future::Future::poll(self.0.as_mut(), cx)
    }
}

/// Wrap another compatible future in an InvokeResult.
#[inline]
pub fn resp<T, E, F>(f: F) -> InvokeResult<T, E>
where
    E: 'static + From<GhostError> + Send,
    F: 'static + std::future::Future<Output = Result<T, E>> + Send,
{
    InvokeResult::new(f)
}

type InnerInvoke<T> = Box<dyn FnOnce(&mut T) + 'static + Send>;
type SendInvoke<T> = futures::channel::mpsc::Sender<InnerInvoke<T>>;

/// GhostActor manages task efficient sequential mutable access
/// to internal state data (type T).
/// GhostActors are `'static` and cheaply clone-able.
/// A clone retains a channel to the same internal state data.
#[derive(Clone)]
pub struct GhostActor<T: 'static + Send>(Arc<SendInvoke<T>>);

impl<T: 'static + Send> GhostActor<T> {
    /// Create a ne GhostActor with initial state.
    pub fn new(mut t: T) -> (Self, ActorDriver) {
        let (send, recv) =
            futures::channel::mpsc::channel::<InnerInvoke<T>>(10);
        let driver =
            ActorDriver(futures::future::FutureExt::boxed(async move {
                // mitigate task thrashing
                let mut recv =
                    futures::stream::StreamExt::ready_chunks(recv, 1024);

                while let Some(invokes) =
                    futures::stream::StreamExt::next(&mut recv).await
                {
                    for invoke in invokes {
                        // give invokes sequential access to mutable state
                        invoke(&mut t);
                    }
                }
            }));

        (Self(Arc::new(send)), driver)
    }

    /// Push state read/mutation logic onto actor queue for processing.
    pub fn invoke<R, E, F>(&self, invoke: F) -> InvokeResult<R, E>
    where
        R: 'static + Send,
        E: 'static + From<GhostError> + Send,
        F: FnOnce(&mut T) -> Result<R, E> + 'static + Send,
    {
        let mut sender: SendInvoke<T> = (*self.0).clone();
        resp(async move {
            // set up oneshot result channel
            let (o_send, o_recv) = futures::channel::oneshot::channel();

            // construct logic closure
            let inner: InnerInvoke<T> = Box::new(move |t: &mut T| {
                let r = invoke(t);
                let _ = o_send.send(r);
            });

            // forward logic closure to actor task driver
            use futures::sink::SinkExt;
            sender.send(inner).await.map_err(GhostError::other)?;

            // await response
            o_recv.await.map_err(GhostError::other)?
        })
    }
}

#[cfg(test)]
mod test;
