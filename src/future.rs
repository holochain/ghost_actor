use crate::*;

/// Result future for GhostActor#invoke().
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct GhostFuture<R, E>(futures::future::BoxFuture<'static, Result<R, E>>)
where
    E: 'static + From<GhostError> + Send;

impl<R, E> GhostFuture<R, E>
where
    E: 'static + From<GhostError> + Send,
{
    /// Wrap another compatible future in an GhostFuture.
    #[inline]
    pub fn new<F>(f: F) -> Self
    where
        F: 'static + std::future::Future<Output = Result<R, E>> + Send,
    {
        Self(futures::future::FutureExt::boxed(f))
    }
}

impl<R, E> std::future::Future for GhostFuture<R, E>
where
    E: 'static + From<GhostError> + Send,
{
    type Output = Result<R, E>;

    #[inline]
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context,
    ) -> std::task::Poll<Self::Output> {
        std::future::Future::poll(self.0.as_mut(), cx)
    }
}

/// Wrap another compatible future in an GhostFuture.
#[inline]
pub fn resp<R, E, F>(f: F) -> GhostFuture<R, E>
where
    E: 'static + From<GhostError> + Send,
    F: 'static + std::future::Future<Output = Result<R, E>> + Send,
{
    GhostFuture::new(f)
}
