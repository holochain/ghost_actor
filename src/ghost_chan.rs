use crate::*;

/// Response callback for an GhostChan message
pub type GhostChanRespond<T> = Box<dyn FnOnce(T) -> GhostResult<()> + 'static + Send>;

/// Sender trait for GhostChan Send subtraits
pub trait GhostChanSend<T: 'static + Send> {
    /// Implement this in your sender newtype to forward GhostChan messages across a
    /// channel.
    fn ghost_chan_send(&mut self, item: T) -> ::must_future::MustBoxFuture<'_, GhostResult<()>>;
}

impl<T: 'static + Send> GhostChanSend<T> for ::futures::channel::mpsc::Sender<T> {
    fn ghost_chan_send(&mut self, item: T) -> ::must_future::MustBoxFuture<'_, GhostResult<()>> {
        use ::futures::{future::FutureExt, sink::SinkExt};

        let send_fut = self.send(item);

        async move {
            send_fut.await?;
            Ok(())
        }
        .boxed()
        .into()
    }
}

/// Container for GhostChan messages
pub struct GhostChanItem<I, O> {
    /// the request input type
    pub input: I,

    /// the response callback for responding to the request
    pub respond: GhostChanRespond<O>,

    /// a tracing span for logically following the request/response
    pub span: tracing::Span,
}

impl<I, O> std::fmt::Debug for GhostChanItem<I, O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GhostChanItem")
    }
}

#[macro_use]
mod ghost_chan_macros;
pub use ghost_chan_macros::*;
