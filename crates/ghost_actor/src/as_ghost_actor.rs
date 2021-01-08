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
    pub trait AsGhostActor: 'static + Send + Sync {
        /// Raw type-erased invoke function.
        /// You probably want to use a higher-level function
        /// with better type safety.
        fn __invoke(
            &self,
            invoke: RawInvokeClosure,
        ) -> GhostFuture<Box<dyn std::any::Any + 'static + Send>, GhostError>;

        /// `BoxGhostActor::clone()` uses this clone internally.
        fn __box_clone(&self) -> BoxGhostActor;

        /// `impl PartialEq for BoxGhostActor` uses this function internally.
        fn __is_same_actor(&self, o: &dyn std::any::Any) -> bool;

        /// `impl Hash for BoxGhostActor` uses this function internally.
        fn __hash_actor(&self, hasher: &mut dyn std::hash::Hasher);

        /// Returns `true` if the channel is still connected to the actor task.
        fn __is_active(&self) -> bool;

        /// Close the channel to the actor task.
        /// This will result in the task being dropped once all pending invocations
        /// have been processed.
        fn __shutdown(&self);
    }

    impl Clone for Box<dyn AsGhostActor> {
        fn clone(&self) -> Self {
            self.__box_clone().0
        }
    }

    impl std::cmp::PartialEq for Box<dyn AsGhostActor> {
        fn eq(&self, o: &Self) -> bool {
            self.__is_same_actor(o)
        }
    }

    impl std::cmp::Eq for Box<dyn AsGhostActor> {}

    impl std::hash::Hash for Box<dyn AsGhostActor> {
        fn hash<Hasher: std::hash::Hasher>(&self, state: &mut Hasher) {
            self.__hash_actor(state);
        }
    }
}

/// Newtype wrapping boxed trait-object version of GhostActor.
#[derive(Clone, Eq)]
pub struct BoxGhostActor(pub Box<dyn AsGhostActor>);

impl std::cmp::PartialEq for BoxGhostActor {
    fn eq(&self, o: &Self) -> bool {
        self.0.eq(&(o.0))
    }
}

impl std::hash::Hash for BoxGhostActor {
    fn hash<Hasher: std::hash::Hasher>(&self, state: &mut Hasher) {
        self.0.hash(state);
    }
}

impl BoxGhostActor {
    /// Get a type-erased BoxGhostActor version of this handle.
    pub fn to_boxed(&self) -> BoxGhostActor {
        self.clone()
    }

    /// Push state read/mutation logic onto actor queue for processing.
    pub fn invoke<T, R, E, F>(&self, invoke: F) -> GhostFuture<R, E>
    where
        T: 'static + Send,
        R: 'static + Send,
        E: 'static + From<GhostError> + Send,
        F: FnOnce(&mut T) -> Result<R, E> + 'static + Send,
    {
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
            let r: R = match a.downcast::<R>() {
                Err(_) => {
                    return Err(
                        GhostError::from("invalid concrete type R").into()
                    )
                }
                Ok(r) => *r,
            };
            Ok(r)
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

    fn __box_clone(&self) -> BoxGhostActor {
        self.0.__box_clone()
    }

    fn __is_same_actor(&self, o: &dyn std::any::Any) -> bool {
        self.0.__is_same_actor(o)
    }

    fn __hash_actor(&self, hasher: &mut dyn std::hash::Hasher) {
        self.0.__hash_actor(hasher);
    }

    fn __is_active(&self) -> bool {
        self.0.__is_active()
    }

    fn __shutdown(&self) {
        self.0.__shutdown();
    }
}
