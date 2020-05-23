//! The `ghost_chan!` macro generates an enum and helper types that make it
//! easy to make inline async requests and await responses.

use crate::*;

/// Response callback for an GhostChan message.
pub type GhostChanRespond<T> = Box<dyn FnOnce(T) -> GhostResult<()> + 'static + Send>;

/// Sender trait for GhostChan Send subtraits.
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

#[macro_use]
mod r#macro;
pub use r#macro::*;
