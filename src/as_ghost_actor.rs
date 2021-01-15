use crate::*;

/// Background utilities for dealing with the `AsGhostActor` trait.
/// For the most part you shouldn't need to deal with these,
/// instead, you should use a concrete type like `GhostActor` or
/// `BoxGhostActor`.
pub mod ghost_actor_trait {
    use super::*;

    /// Closure definition for AsGhostActor::__invoke
    pub type RawInvokeClosure = Box<
        dyn FnOnce(
                &mut dyn std::any::Any,
            ) -> Result<
                Box<dyn std::any::Any + 'static + Send>,
                GhostError,
            >
            + 'static
            + Send,
    >;

    /// Generic GhostActor Trait. You shouldn't need to deal with this
    /// unless you are implementing an alternate ghost_actor backend.
    ///
    /// This trait allows:
    /// - potential alternate ghost_actor implementations
    /// - usage of `impl AsGhostActor` in functions / generics
    /// - type erasure via `BoxGhostActor`
    pub trait AsGhostActor: 'static + Send + Sync + std::fmt::Debug {
        /// Raw type-erased invoke function.
        /// You probably want to use a higher-level function
        /// with better type safety.
        fn __invoke(
            &self,
            invoke: RawInvokeClosure,
        ) -> GhostFuture<Box<dyn std::any::Any + 'static + Send>, GhostError>;

        /// Returns `true` if the channel is still connected to the actor task.
        fn __is_active(&self) -> bool;

        /// Close the channel to the actor task.
        /// This will result in the task being dropped once all pending invocations
        /// have been processed.
        fn __shutdown(&self);

        ghost_box_trait_fns!(AsGhostActor);
    }
    ghost_box_trait!(AsGhostActor);
}

/// Newtype wrapping boxed type-erased trait-object version of GhostActor.
/// Prefer using the strongly typed `GhostActor<T>`. This boxed type allows,
/// for example, placing differing typed BoxGhostActor instances in a
/// HashSet<BoxGhostActor> if you have some external mechanism for determining
/// type `T` when calling `invoke()`.
#[derive(Debug)]
pub struct BoxGhostActor(pub Box<dyn AsGhostActor>);
ghost_box_new_type!(BoxGhostActor);

impl BoxGhostActor {
    /// Push state read/mutation logic onto actor queue for processing.
    pub fn invoke<T, R, E, F>(&self, invoke: F) -> GhostFuture<R, E>
    where
        T: 'static + Send,
        R: 'static + Send,
        E: 'static + From<GhostError> + Send,
        F: FnOnce(&mut T) -> Result<R, E> + 'static + Send,
    {
        // NOTE - we don't have to do any tracing trickery here
        //        it can all be handled by the concrete implementation
        //        of __invoke

        let inner = Box::new(move |a: &mut dyn std::any::Any| {
            let t: &mut T = match a.downcast_mut() {
                None => {
                    return Err(GhostError::from("invalid concrete type T"));
                }
                Some(t) => t,
            };
            let r: Box<dyn std::any::Any + 'static + Send> =
                Box::new(invoke(t));
            Ok(r)
        });

        let fut = self.__invoke(inner);

        resp(async move {
            let a: Box<dyn std::any::Any> = fut.await?;
            let r: Result<R, E> = match a.downcast() {
                Err(_) => {
                    return Err(
                        GhostError::from("invalid concrete type R").into()
                    )
                }
                Ok(r) => *r,
            };
            r
        })
    }

    /// Returns `true` if the channel is still connected to the actor task.
    pub fn is_active(&self) -> bool {
        self.__is_active()
    }

    /// Close the channel to the actor task.
    /// This will result in the task being dropped once all pending invocations
    /// have been processed.
    pub fn shutdown(&self) {
        self.__shutdown();
    }
}

impl AsGhostActor for BoxGhostActor {
    fn __invoke(
        &self,
        invoke: RawInvokeClosure,
    ) -> GhostFuture<Box<dyn std::any::Any + 'static + Send>, GhostError> {
        self.0.__invoke(invoke)
    }

    fn __is_active(&self) -> bool {
        self.0.__is_active()
    }

    fn __shutdown(&self) {
        self.0.__shutdown();
    }

    ghost_box_trait_impl_fns!(AsGhostActor);
}
