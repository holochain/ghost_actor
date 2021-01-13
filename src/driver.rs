/// Driver future representing an actor task.
/// Please spawn this into whatever executor framework you are using.
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct GhostDriver(pub(crate) futures::future::BoxFuture<'static, ()>);

impl std::future::Future for GhostDriver {
    type Output = ();

    #[inline]
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context,
    ) -> std::task::Poll<Self::Output> {
        std::future::Future::poll(self.0.as_mut(), cx)
    }
}
