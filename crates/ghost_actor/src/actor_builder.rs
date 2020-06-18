//! Us GhostActorBuilder to construct ghost actor tasks.

use crate::*;
use futures::{
    sink::SinkExt,
    stream::{BoxStream, StreamExt},
};
use std::sync::Arc;

/// This struct controls how a running actor functions.
/// If you wish to implement your own GhostChannelSender, you'll use this
/// to control the actor at the receiving end.
#[derive(Clone)]
pub struct GhostActorControl {
    interupt_send: futures::channel::mpsc::Sender<()>,
    state: Arc<GhostActorState>,
}

impl GhostActorControl {
    /// Internal constructed by GhostActorBuilder
    pub(crate) fn new(
        interupt_send: futures::channel::mpsc::Sender<()>,
    ) -> Self {
        Self {
            interupt_send,
            state: Arc::new(GhostActorState::new()),
        }
    }

    /// Shutdown the actor once all pending messages have been processed.
    /// Future completes when the actor is shutdown.
    pub fn ghost_actor_shutdown(&self) -> GhostFuture<()> {
        self.state.set_pending_shutdown();
        //must_future::MustBoxFuture::new(async move { Ok(()) })
        // TODO - yeah, not really sure how this will actually work
        //        just calling into shutdown_immediate for now
        self.ghost_actor_shutdown_immediate()
    }

    /// Shutdown the actor immediately. All pending tasks will error.
    pub fn ghost_actor_shutdown_immediate(&self) -> GhostFuture<()> {
        self.state.set_shutdown();
        let mut i_send = self.interupt_send.clone();
        must_future::MustBoxFuture::new(async move {
            let _ = i_send.send(()).await;
            Ok(())
        })
    }

    /// Returns true if the receiving actor is still running.
    pub fn ghost_actor_active(&self) -> bool {
        self.state.get() == GhostActorStateType::Active
    }
}

/// Allows attaching new GhostEvent channels to a GhostActor task.
pub struct GhostActorChannelFactory<H: GhostControlHandler> {
    inject: InjectLock<H>,
    interupt_send: futures::channel::mpsc::Sender<()>,
    control: Arc<crate::actor_builder::GhostActorControl>,
}

impl<H: GhostControlHandler> GhostActorChannelFactory<H> {
    pub(crate) fn new(
        control: Arc<crate::actor_builder::GhostActorControl>,
        interupt_send: futures::channel::mpsc::Sender<()>,
    ) -> (Self, InjectLock<H>) {
        let inject = InjectLock::new();
        (
            Self {
                inject: inject.clone(),
                interupt_send,
                control,
            },
            inject,
        )
    }

    /// Attach a new event sender to a running (or pending build) GhostActor.
    /// Note - you should only call this once for each GhostEvent type.
    /// If you want multiple senders for a GhostEvent, clone the resulting
    /// Sender.
    pub fn create_channel<E>(&self) -> GhostFuture<GhostSender<E>>
    where
        E: GhostEvent + GhostDispatch<H>,
        H: GhostControlHandler + GhostHandler<E>,
    {
        let (ghost_sender, receiver) =
            <GhostSender<E>>::new(self.control.clone());

        // this unifies the various incoming event types
        // into the same handler injector type so we can multiplex in the actor
        let stream: BoxStream<'static, GhostActorInject<H>> =
            Box::pin(receiver.map(|event| {
                let inject: GhostActorInject<H> = Box::new(move |handler| {
                    handler.ghost_actor_dispatch(event);
                });
                inject
            }));

        let push_fut = self.inject.push(stream);
        let mut i_send = self.interupt_send.clone();
        must_future::MustBoxFuture::new(async move {
            push_fut.await?;
            let _ = i_send.send(()).await;
            Ok(ghost_sender)
        })
    }
}

impl<H: GhostControlHandler> Clone for GhostActorChannelFactory<H> {
    fn clone(&self) -> Self {
        Self {
            inject: self.inject.clone(),
            interupt_send: self.interupt_send.clone(),
            control: self.control.clone(),
        }
    }
}

/// Construct a GhostActor by specifying which GhostEvents it handles.
/// GhostSenders can also attach additional senders post-spawn, if the
/// handler supports the given GhostEvent.
pub struct GhostActorBuilder<H: GhostControlHandler> {
    control: Arc<GhostActorControl>,
    channel_factory: GhostActorChannelFactory<H>,
    inject: InjectLock<H>,
    interupt_recv: futures::channel::mpsc::Receiver<()>,
}

impl<H: GhostControlHandler> Default for GhostActorBuilder<H> {
    fn default() -> Self {
        Self::new()
    }
}

impl<H: GhostControlHandler> GhostActorBuilder<H> {
    /// Start here to create a new GhostActor task.
    pub fn new() -> Self {
        let (interupt_send, interupt_recv) =
            futures::channel::mpsc::channel::<()>(10);
        let control = Arc::new(GhostActorControl::new(interupt_send.clone()));
        let (channel_factory, inject) =
            GhostActorChannelFactory::new(control.clone(), interupt_send);
        Self {
            control,
            channel_factory,
            inject,
            interupt_recv,
        }
    }

    /// To add GhostSenders to the new actor, you need access to the
    /// channel factory.
    /// Pro Tip: You can cheaply clone this factory
    /// and keep it around for later : )
    pub fn channel_factory(&self) -> &GhostActorChannelFactory<H> {
        &self.channel_factory
    }

    /// Pass in your handler item and start the actor task loop.
    pub fn spawn(self, mut handler: H) -> GhostFuture<()> {
        let GhostActorBuilder {
            control,
            inject,
            interupt_recv,
            ..
        } = self;

        let mut stream_multiplexer = <futures::stream::SelectAll<
            BoxStream<'static, GhostActorInject<H>>,
        >>::new();

        let interupt_stream: BoxStream<'static, GhostActorInject<H>> =
            Box::pin(interupt_recv.map(|_| {
                let inject: GhostActorInject<H> = Box::new(|_| {});
                inject
            }));
        stream_multiplexer.push(interupt_stream);

        must_future::MustBoxFuture::new(async move {
            loop {
                // Before we await on the injector lock,
                // make sure we are still supposed to be running.
                if !control.ghost_actor_active() {
                    break;
                }

                // Check if we have any new streams to inject.
                for i in inject.drain().await? {
                    stream_multiplexer.push(i);
                }

                // Before we await on the multiplexer stream,
                // make sure we are still supposed to be running.
                if !control.ghost_actor_active() {
                    break;
                }

                // Check if we have any incoming messages to process.
                // Note - This multiplexer also processes the "interupt"
                //        stream, which doesn't do anything to the handler,
                //        but lets us check our control/inject items.
                match stream_multiplexer.next().await {
                    Some(i) => i(&mut handler),
                    None => break,
                }
            }
            control.state.set_shutdown();

            // finally - invoke the shutdown handler
            //           allows actor to cleanup / do any final triggers
            handler.ghost_actor_shutdown();

            Ok(())
        })
    }
}

// -- private -- //

pub(crate) type GhostActorInject<H> = Box<dyn FnOnce(&mut H) + 'static + Send>;

/// internal inject new streams into our actor multiplexer
/// note - using a mutex here instead of a channel
///        because we want to process them asap - not wait in the queue.
pub(crate) struct InjectLock<H: GhostControlHandler>(
    Arc<futures::lock::Mutex<Vec<BoxStream<'static, GhostActorInject<H>>>>>,
);

impl<H: GhostControlHandler> Clone for InjectLock<H> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<H: GhostControlHandler> InjectLock<H> {
    pub fn new() -> Self {
        Self(Arc::new(futures::lock::Mutex::new(Vec::new())))
    }

    pub fn push(
        &self,
        i: BoxStream<'static, GhostActorInject<H>>,
    ) -> GhostFuture<()> {
        let lock = self.0.clone();
        must_future::MustBoxFuture::new(async move {
            let mut g = lock.lock().await;
            g.push(i);
            Ok(())
        })
    }

    pub fn drain(
        &self,
    ) -> GhostFuture<Vec<BoxStream<'static, GhostActorInject<H>>>> {
        let lock = self.0.clone();
        must_future::MustBoxFuture::new(async move {
            let mut g = lock.lock().await;
            let out = g.drain(..).collect();
            Ok(out)
        })
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq)]
pub(crate) enum GhostActorStateType {
    Active = 0x00,
    PendingShutdown = 0xfe,
    Shutdown = 0xff,
}

impl From<u8> for GhostActorStateType {
    fn from(u: u8) -> Self {
        match u {
            0x00 => GhostActorStateType::Active,
            0xfe => GhostActorStateType::PendingShutdown,
            0xff => GhostActorStateType::Shutdown,
            _ => panic!("corrupt GhostActorStateType"),
        }
    }
}

pub(crate) struct GhostActorState(std::sync::atomic::AtomicU8);

impl GhostActorState {
    pub fn new() -> Self {
        Self(std::sync::atomic::AtomicU8::new(
            GhostActorStateType::Active as u8,
        ))
    }

    pub fn set_pending_shutdown(&self) {
        self.0.store(
            GhostActorStateType::PendingShutdown as u8,
            std::sync::atomic::Ordering::SeqCst,
        );
    }

    pub fn set_shutdown(&self) {
        self.0.store(
            GhostActorStateType::Shutdown as u8,
            std::sync::atomic::Ordering::SeqCst,
        );
    }

    pub fn get(&self) -> GhostActorStateType {
        self.0.load(std::sync::atomic::Ordering::SeqCst).into()
    }
}
