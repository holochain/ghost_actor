use crate::*;
use std::sync::Arc;

type InnerInvoke<T> = Box<dyn FnOnce(&mut T) + 'static + Send>;
type SendInvoke<T> = futures::channel::mpsc::Sender<InnerInvoke<T>>;

/// GhostActor manages task efficient sequential mutable access
/// to internal state data (type T).
/// GhostActors are `'static` and cheaply clone-able.
/// A clone retains a channel to the same internal state data.
pub struct GhostActor<T: 'static + Send>(Arc<SendInvoke<T>>);

impl<T: 'static + Send> GhostActor<T> {
    /// Create a new GhostActor with default config and initial state.
    pub fn new(t: T) -> (Self, GhostDriver) {
        Self::new_config(GhostConfig::default(), t)
    }

    /// Create a new GhostActor with config and initial state.
    pub fn new_config(config: GhostConfig, t: T) -> (Self, GhostDriver) {
        let mut t = t;

        let (send, recv) = futures::channel::mpsc::channel::<InnerInvoke<T>>(
            config.channel_bound,
        );

        let driver =
            GhostDriver(futures::future::FutureExt::boxed(async move {
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

    /// Get a type-erased BoxGhostActor version of this handle.
    pub fn to_boxed(&self) -> BoxGhostActor {
        self.__box_clone()
    }

    /// Push state read/mutation logic onto actor queue for processing.
    pub fn invoke<R, E, F>(&self, invoke: F) -> GhostFuture<R, E>
    where
        R: 'static + Send,
        E: 'static + From<GhostError> + Send,
        F: FnOnce(&mut T) -> Result<R, E> + 'static + Send,
    {
        let mut sender = (*self.0).clone();
        //let mut sender: SendInvoke<T> = (*self.0).clone();
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

    /// Returns `true` if the channel is still connected to the actor task.
    pub fn is_active(&self) -> bool {
        !self.0.is_closed()
    }

    /// Close the channel to the actor task.
    /// This will result in the task being dropped once all pending invocations
    /// have been processed.
    pub fn shutdown(&self) {
        (*self.0).clone().close_channel();
    }
}

impl<T: 'static + Send> AsGhostActor for GhostActor<T> {
    fn __invoke(
        &self,
        invoke: RawInvokeClosure,
    ) -> GhostFuture<Box<dyn std::any::Any + 'static + Send>, GhostError> {
        let fut = self.invoke(|t| invoke(t));
        resp(async move { fut.await })
    }

    fn __box_clone(&self) -> BoxGhostActor {
        BoxGhostActor(Box::new(self.clone()))
    }

    fn __is_same_actor(&self, o: &dyn std::any::Any) -> bool {
        let o: &GhostActor<T> = match std::any::Any::downcast_ref(o) {
            None => return false,
            Some(o) => o,
        };
        self.0.same_receiver(&o.0)
    }

    fn __hash_actor(&self, hasher: &mut dyn std::hash::Hasher) {
        self.0.hash_receiver(&mut Box::new(hasher));
    }

    fn __is_active(&self) -> bool {
        GhostActor::is_active(self)
    }

    fn __shutdown(&self) {
        GhostActor::shutdown(self);
    }
}

impl<T: 'static + Send> std::clone::Clone for GhostActor<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: 'static + Send> std::cmp::PartialEq for GhostActor<T> {
    fn eq(&self, o: &Self) -> bool {
        self.0.same_receiver(&o.0)
    }
}

impl<T: 'static + Send> std::cmp::Eq for GhostActor<T> {}

impl<T: 'static + Send> std::hash::Hash for GhostActor<T> {
    fn hash<Hasher: std::hash::Hasher>(&self, state: &mut Hasher) {
        self.0.hash_receiver(state);
    }
}
