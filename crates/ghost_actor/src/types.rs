/// Ghost error type.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GhostError {
    /// GhostActorDisconnected
    #[error("GhostActorDisconnected")]
    Disconnected,

    /// There was a timeout trying to send an event
    #[error("Timeout")]
    Timeout,

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

impl From<futures::channel::mpsc::SendError> for GhostError {
    fn from(_: futures::channel::mpsc::SendError) -> Self {
        Self::Disconnected
    }
}

impl From<futures::channel::oneshot::Canceled> for GhostError {
    fn from(_: futures::channel::oneshot::Canceled) -> Self {
        Self::Disconnected
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
    // ::futures::channel::oneshot::Sender<(T, ::observability::Context)>,
    ::futures::channel::oneshot::Sender<(T, ())>,
    &'static str,
);

impl<T: 'static + Send> GhostRespond<T> {
    #[doc(hidden)]
    pub fn new(
        sender: ::futures::channel::oneshot::Sender<(
            T,
            // ::observability::Context,
            (),
        )>,
        trace: &'static str,
    ) -> Self {
        Self(sender, trace)
    }

    /// Call this to respond to a ghost request.
    pub fn respond(self, t: T) {
        // use observability::OpenSpanExt;
        // In a ghost channel, the only error you can get is that the sender
        // is no longer available to receive the response.
        // As a responder, we don't care.
        // let context = tracing::Span::get_current_context();
        // let _ = self.0.send((t, context));
        let _ = self.0.send((t, ()));
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

/// All handlers must implement these generic control callbacks.
/// Many of the functions within are provided as no-ops that can be overridden.
pub trait GhostControlHandler: 'static + Send + Sized {
    /// Called when the actor task loops ends.
    /// Allows for any needed cleanup / triggers.
    fn handle_ghost_actor_shutdown(
        self,
    ) -> must_future::MustBoxFuture<'static, ()> {
        // default no-op
        must_future::MustBoxFuture::new(async move {})
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

impl<E: GhostEvent> GhostChannelSender<E>
    for futures::channel::mpsc::Sender<E>
{
    fn ghost_actor_channel_send(&self, event: E) -> GhostFuture<()> {
        let mut sender = self.clone();
        ::must_future::MustBoxFuture::new(async move {
            match tokio::time::timeout(
                std::time::Duration::from_secs(10),
                futures::sink::SinkExt::send(&mut sender, event),
            )
            .await
            {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => Err(e.into()),
                Err(_) => Err(GhostError::Timeout),
            }
        })
    }
}

/// A full sender that can control the actor side of the channel.
pub trait GhostControlSender<E: GhostEvent>: GhostChannelSender<E> {
    /// Shutdown the actor once all pending messages have been processed.
    /// Future completes when the actor is shutdown.
    fn ghost_actor_shutdown(&self) -> GhostFuture<()>;

    /// Shutdown the actor immediately. All pending tasks will error.
    fn ghost_actor_shutdown_immediate(&self) -> GhostFuture<()>;

    /// Returns true if the receiving actor is still running.
    fn ghost_actor_is_active(&self) -> bool;
}

/// A provided GhostSender (impl GhostChannelSender) implementation.
pub struct GhostSender<E: GhostEvent>(
    ::futures::channel::mpsc::Sender<E>,
    std::sync::Arc<crate::actor_builder::GhostActorControl>,
);

impl<E: GhostEvent> GhostSender<E> {
    pub(crate) fn new(
        ghost_actor_control: std::sync::Arc<
            crate::actor_builder::GhostActorControl,
        >,
    ) -> (Self, GhostReceiver<E>) {
        let (s, r) = ::futures::channel::mpsc::channel(10);
        (
            GhostSender(s, ghost_actor_control),
            GhostReceiver(Box::new(r)),
        )
    }
}

impl<E: GhostEvent> ::std::clone::Clone for GhostSender<E> {
    fn clone(&self) -> Self {
        GhostSender(self.0.clone(), self.1.clone())
    }
}

impl<E: GhostEvent> ::std::cmp::PartialEq for GhostSender<E> {
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
        self.0.ghost_actor_channel_send(event)
    }
}

impl<E: GhostEvent> GhostControlSender<E> for GhostSender<E> {
    fn ghost_actor_shutdown(&self) -> GhostFuture<()> {
        self.1.ghost_actor_shutdown()
    }

    fn ghost_actor_shutdown_immediate(&self) -> GhostFuture<()> {
        self.1.ghost_actor_shutdown_immediate()
    }

    fn ghost_actor_is_active(&self) -> bool {
        self.1.ghost_actor_is_active()
    }
}

/// Indicates an item is the Receiver side of a channel that can
/// forward/handle GhostEvents.
pub trait GhostChannelReceiver<E: GhostEvent>:
    'static + Send + Sized + ::futures::stream::Stream<Item = E>
{
}

impl<E: GhostEvent> GhostChannelReceiver<E>
    for ::futures::channel::mpsc::Receiver<E>
{
}

/// RAII guard for spawning a mock handler that will be dropped appropriately
/// at the end of a test to trigger any needed expect panics, etc.
#[cfg(feature = "test_utils")]
pub struct MockHandler<E, H>
where
    E: GhostEvent + GhostDispatch<H>,
    H: GhostControlHandler + GhostHandler<E>,
{
    ghost_sender: GhostSender<E>,
    chan_sender: futures::channel::mpsc::Sender<E>,
    driver_fut: Option<futures::future::BoxFuture<'static, ()>>,
    _phantom: std::marker::PhantomData<&'static H>,
}

#[cfg(feature = "test_utils")]
impl<E, H> Drop for MockHandler<E, H>
where
    E: GhostEvent + GhostDispatch<H>,
    H: GhostControlHandler + GhostHandler<E>,
{
    fn drop(&mut self) {
        let fut = self.ghost_sender.ghost_actor_shutdown_immediate();
        let driver_fut = self.driver_fut.take();

        // this block-on is here for mock testing purposes only.
        // We need to make sure any panics that are triggered
        // by the mock implementation are bubbled back up to the test thread.
        futures::executor::block_on(async move {
            fut.await.unwrap();
            if let Some(driver_fut) = driver_fut {
                driver_fut.await;
            }
        });
    }
}

#[cfg(feature = "test_utils")]
impl<E, H> MockHandler<E, H>
where
    E: GhostEvent + GhostDispatch<H>,
    H: GhostControlHandler + GhostHandler<E>,
{
    /// Construct & spawn a new actor based on given mock handler.
    /// See `get_ghost_sender` or `get_chan_sender` for usage.
    pub async fn spawn<R1, R2, F, S>(handler: H, spawn: S) -> Self
    where
        R1: std::fmt::Debug,
        R2: std::fmt::Debug,
        F: std::future::Future<Output = Result<Result<(), R1>, R2>>
            + 'static
            + Send,
        S: Fn(futures::future::BoxFuture<'static, Result<(), GhostError>>) -> F,
    {
        let builder = crate::actor_builder::GhostActorBuilder::new();

        let ghost_sender =
            builder.channel_factory().create_channel().await.unwrap();

        let (chan_sender, r) = futures::channel::mpsc::channel(4096);

        builder.channel_factory().attach_receiver(r).await.unwrap();

        let fut = spawn(futures::future::FutureExt::boxed(async move {
            builder.spawn(handler).await
        }));
        let driver_fut = Some(futures::future::FutureExt::boxed(async move {
            fut.await.unwrap().unwrap();
        }));

        Self {
            ghost_sender,
            chan_sender,
            driver_fut,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Fetch a ghost_sender attached to the handler spawned by this guard.
    pub fn get_ghost_sender(&self) -> GhostSender<E> {
        self.ghost_sender.clone()
    }

    /// Fetch a chan_sender attached to the handler spawned by this guard.
    pub fn get_chan_sender(&self) -> futures::channel::mpsc::Sender<E> {
        self.chan_sender.clone()
    }
}

// -- private -- //

/// internal GhostReceiver (impl GhostChannelReceiver) implementation.
pub(crate) struct GhostReceiver<E: GhostEvent>(
    Box<::futures::channel::mpsc::Receiver<E>>,
);

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
