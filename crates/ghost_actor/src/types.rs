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

/// Ghost Future Result Type.
pub type GhostFuture<T> = ::must_future::MustBoxFuture<'static, GhostResult<T>>;

/// This future represents a spawned GhostActor task, you must await
/// or spawn this task into an executor for the actor to function.
pub type GhostActorDriver = ::must_future::MustBoxFuture<'static, ()>;

/// Response callback for ghost request.
#[must_use]
pub struct GhostRespond<T: 'static + Send>(
    ::futures::channel::oneshot::Sender<(T, ::tracing::Span)>,
    &'static str,
);

impl<T: 'static + Send> GhostRespond<T> {
    #[doc(hidden)]
    pub fn new(
        sender: ::futures::channel::oneshot::Sender<(T, ::tracing::Span)>,
        trace: &'static str,
    ) -> Self {
        Self(sender, trace)
    }

    /// Call this to respond to a ghost request.
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
impl<T: 'static + Send> std::ops::FnOnce<(T,)> for GhostRespond<T> {
    type Output = ();
    extern "rust-call" fn call_once(self, args: (T,)) -> Self::Output {
        self.respond(args.0)
    }
}

/// A message that can be sent over a GhostEvent channel.
pub trait GhostEvent: 'static + Send + Sized {}

/// An upgraded GhostEvent that knows how to dispatch to a handler.
pub trait GhostDispatch<H: GhostHandler<Self>>: GhostEvent {
    /// Process a dispatch event with a given GhostHandler.
    fn ghost_actor_dispatch(self, h: &mut H);
}

/// An item that can handle an incoming GhostEvent.
pub trait GhostHandler<D: GhostDispatch<Self>>: 'static + Send + Sized {
    /// Process a dispatch event with this GhostHandler.
    fn ghost_actor_dispatch(&mut self, d: D) {
        d.ghost_actor_dispatch(self);
    }
}

/// Indicates an item is the Sender side of a channel that can
/// forward/handle GhostEvents.
pub trait GhostChannelSender<E: GhostEvent>:
    'static + Send + Sync + Sized + Clone
{
    /// Forward a GhostEvent along this channel.
    fn ghost_actor_channel_send(&self, event: E) -> GhostFuture<()>;
}

/// Indicates an item is the Receiver side of a channel that can
/// forward/handle GhostEvents.
pub trait GhostChannelReceiver<E: GhostEvent>:
    'static + Send + Sized + ::futures::stream::Stream<Item = E>
{
}

/// A provided GhostSender (impl GhostChannelSender) implementation.
pub struct GhostSender<E: GhostEvent>(
    ::futures::channel::mpsc::Sender<E>,
);

impl<E: GhostEvent> ::std::clone::Clone for GhostSender<E> {
    fn clone(&self) -> Self {
        GhostSender(self.0.clone())
    }
}

impl< E: GhostEvent> ::std::cmp::PartialEq for GhostSender<E> {
    fn eq(&self, o: &Self) -> bool {
        self.0.same_receiver(&o.0)
    }
}

impl<E: GhostEvent> ::std::cmp::Eq for GhostSender<E> {}

impl<E: GhostEvent> ::std::hash::Hash for GhostSender<E> {
    fn hash<Hasher: ::std::hash::Hasher>(&self, state: &mut Hasher) {
        self.0.hash_receiver(state);
    }
}

impl<E: GhostEvent> GhostChannelSender<E> for GhostSender<E> {
    fn ghost_actor_channel_send(&self, event: E) -> GhostFuture<()> {
        let mut sender = self.0.clone();
        ::must_future::MustBoxFuture::new(async move {
            futures::sink::SinkExt::send(&mut sender, event).await?;
            Ok(())
        })
    }
}

/// A provided GhostReceiver (impl GhostChannelReceiver) implementation.
pub struct GhostReceiver<E: GhostEvent>(Box<
    ::futures::channel::mpsc::Receiver<E>,
>);

impl<E: GhostEvent> ::futures::stream::Stream for GhostReceiver<E> {
    type Item = E;

    fn poll_next(
        self: ::std::pin::Pin<&mut Self>,
        cx: &mut ::std::task::Context,
    ) -> ::std::task::Poll<Option<Self::Item>> {
        let p = ::std::pin::Pin::new(&mut (self.get_mut().0));
        ::futures::stream::Stream::poll_next(p, cx)
    }
}

impl<E: GhostEvent> GhostChannelReceiver<E> for GhostReceiver<E> {}

/// Spawn a new GhostChannel send/receive pair.
pub fn spawn_ghost_channel<E: GhostEvent>(
) -> (GhostSender<E>, GhostReceiver<E>) {
    let (s, r) = ::futures::channel::mpsc::channel(10);
    (GhostSender(s), GhostReceiver(Box::new(r)))
}
