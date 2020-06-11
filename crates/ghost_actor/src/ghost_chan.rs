//! The `ghost_chan!` macro generates an enum and helper types that make it
//! easy to make inline async requests and await responses.

use crate::*;

/// Response callback for an GhostChan message.
#[must_use]
pub struct GhostChanRespond<T: 'static + Send>(
    ::futures::channel::oneshot::Sender<(T, ::tracing::Span)>,
    &'static str,
);

impl<T: 'static + Send> GhostChanRespond<T> {
    #[doc(hidden)]
    pub fn new(
        sender: ::futures::channel::oneshot::Sender<(T, ::tracing::Span)>,
        trace: &'static str,
    ) -> Self {
        Self(sender, trace)
    }

    /// Call this to respond to a GhostChan message.
    pub fn respond(self, t: T) {
        // In a ghost channel, the only error you can get is that the sender
        // is no longer available to receive the response.
        // As a responder, we don't care.
        let _ = self
            .0
            .send((t, tracing::debug_span!("respond", "{}", self.1)));
    }

    /// For those who simply cannot stand typing `respond.respond()`,
    /// here is a shortcut.
    pub fn r(self, t: T) {
        self.respond(t);
    }
}

// alas! implementing FnOnce is unstable... so most folks will have to suffer
// the double `respond.respond(bla)`
#[cfg(feature = "unstable")]
impl<T: 'static + Send> std::ops::FnOnce<(T,)> for GhostChanRespond<T> {
    type Output = ();
    extern "rust-call" fn call_once(self, args: (T,)) -> Self::Output {
        self.respond(args.0)
    }
}

/// Sender trait for GhostChan Send subtraits.
pub trait GhostChanSend<T: 'static + Send> {
    /// Implement this in your sender newtype to forward GhostChan messages across a
    /// channel.
    fn ghost_chan_send(
        &mut self,
        item: T,
    ) -> ::must_future::MustBoxFuture<'_, GhostResult<()>>;
}

impl<T: 'static + Send> GhostChanSend<T>
    for ::futures::channel::mpsc::Sender<T>
{
    fn ghost_chan_send(
        &mut self,
        item: T,
    ) -> ::must_future::MustBoxFuture<'_, GhostResult<()>> {
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
