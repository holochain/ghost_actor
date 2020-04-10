/// The main workhorse macro for constructing GhostActors.
/// This will define the protocol for building GhostActors.
#[macro_export]
macro_rules! ghost_actor {
    // using @inner_ self references so we don't have to export / pollute
    // a bunch of sub macros.

    // -- public api arms -- //

    (
        name: $name:ident,
        error: $error:ty,
        api: { $( $req_name:ident :: $req_fname:ident ( $doc:expr, $req_type:ty, $res_type:ty ) ),* }
    ) => {
        $crate::ghost_actor! { @inner
            (), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type ),*
        }
    };

    (
        name: $name:ident,
        error: $error:ty,
        api: { $( $req_name:ident :: $req_fname:ident ( $doc:expr, $req_type:ty, $res_type:ty ) ),*, }
    ) => {
        $crate::ghost_actor! { @inner
            (), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type ),*
        }
    };

    (
        name: pub $name:ident,
        error: $error:ty,
        api: { $( $req_name:ident :: $req_fname:ident ( $doc:expr, $req_type:ty, $res_type:ty ) ),*, }
    ) => {
        $crate::ghost_actor! { @inner
            (pub), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type ),*
        }
    };

    (
        name: pub $name:ident,
        error: $error:ty,
        api: { $( $req_name:ident :: $req_fname:ident ( $doc:expr, $req_type:ty, $res_type:ty ) ),* }
    ) => {
        $crate::ghost_actor! { @inner
            (pub), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type ),*
        }
    };

    // -- "inner" arm dispatches to further individual inner arm helpers -- //

    ( @inner
        ($($vis:tt)*), $name:ident, $error:ty,
        $( $doc:expr, $req_name:ident, $req_fname:ident, $req_type:ty, $res_type:ty ),*
    ) => {
        $crate::ghost_chan! { @inner
            ($($vis)*), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type ),*,
            "custom", GhostActorCustom, ghost_actor_custom, Box<dyn ::std::any::Any + 'static + Send>, (),
            "internal", GhostActorInternal, ghost_actor_internal, Box<dyn ::std::any::Any + 'static + Send>, (),
            "shutdown", GhostActorShutdown, ghost_actor_shutdown, (), ()
        }
        $crate::ghost_actor! { @inner_handler
            ($($vis)*), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type ),*
        }
        $crate::ghost_actor! { @inner_sender
            ($($vis)*), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type ),*
        }
        /*
        $crate::ghost_actor! { @inner_internal_sender
            ($($vis)*), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type ),*
        }
        */
    };

    // -- "handler" arm writes our handler trait -- //

    ( @inner_handler
        ($($vis:tt)*), $name:ident, $error:ty,
        $( $doc:expr, $req_name:ident, $req_fname:ident, $req_type:ty, $res_type:ty ),*
    ) => {
        paste::item! {
            #[doc = "Implement this trait to process incoming actor messages."]
            $($vis)* trait [< $name Handler >] <
                C: $crate::GhostRequestType,
                I: $crate::GhostRequestType,
            > : 'static + Send {
                // -- api handlers -- //

                $(
                    #[doc = $doc]
                    fn [< handle_ $req_fname >] (
                        &mut self, /*internal_sender: &mut [< $name InternalSender >] <C, I>,*/ input: $req_type
                    ) -> ::std::result::Result<$res_type, $error>;
                )*

                // -- provided -- //

                #[allow(unused_variables)]
                #[doc = "Handle custom messages specific to this exact actor implementation. The provided implementation panics with unimplemented!"]
                fn handle_ghost_actor_custom(
                    &mut self, /*internal_sender: &mut [< $name InternalSender >] <C, I>,*/ input: C,
                ) -> ::std::result::Result<
                    C::ResponseType,
                    $error,
                > {
                    unimplemented!()
                }

                #[allow(unused_variables)]
                #[doc = "Handle internal messages specific to this exact actor implementation. The provided implementation panics with unimplemented!"]
                fn handle_ghost_actor_internal(
                    &mut self, /*internal_sender: &mut [< $name InternalSender >] <C, I>,*/ input: I,
                ) -> ::std::result::Result<
                    I::ResponseType,
                    $error,
                > {
                    unimplemented!()
                }
            }
        }
    };

    // -- "sender" arm writes our sender struct -- //

    ( @inner_sender
        ($($vis:tt)*), $name:ident, $error:ty,
        $( $doc:expr, $req_name:ident, $req_fname:ident, $req_type:ty, $res_type:ty ),*
    ) => {
        paste::item! {
            #[doc = "Helper for ghost_actor Sender custom."]
            $($vis)* struct [< $name CustomSender >] <'lt, C>
            where
                C: 'static + Send,
            {
                sender: &'lt mut ::futures::channel::mpsc::Sender<$name>,
                phantom: ::std::marker::PhantomData<C>,
            }

            impl<C> $crate::GhostChanSend<C> for [< $name CustomSender >] <'_, C>
            where
                C: 'static + Send,
            {
                fn ghost_chan_send(&mut self, item: C) -> ::must_future::MustBoxFuture<'_, $crate::GhostResult<()>> {
                    use ::futures::{future::FutureExt, sink::SinkExt};

                    let input: Box<dyn ::std::any::Any + Send> = Box::new(item);

                    // this item (if it is a GhostChan) already encapsulates
                    // the response handling - send dummy no-op respond
                    let item = $crate::GhostChanItem {
                        input,
                        respond: Box::new(|_| Ok(())),
                        span: tracing::trace_span!("noop"),
                    };

                    let send_fut = self.sender.send(
                        $name::GhostActorCustom(item)
                    );

                    async move {
                        send_fut.await?;
                        Ok(())
                    }
                    .boxed()
                    .into()
                }
            }

            #[doc = "A cheaply clone-able handle to control a ghost_actor task."]
            #[derive(Clone)]
            $($vis)* struct [< $name Sender >] <C>
            where
                C: 'static + Send,
            {
                sender: ::futures::channel::mpsc::Sender<$name>,
                phantom: ::std::marker::PhantomData<C>,
            }

            impl<C> [< $name Sender >] <C>
            where
                C: 'static + Send,
            {
                $(
                    #[doc = $doc]
                    pub async fn $req_fname (
                        &mut self, input: $req_type,
                    ) -> ::std::result::Result<$res_type, $error> {
                        use [< $name Send >];

                        self.sender. $req_fname (input) .await
                    }
                )*

                /// Send a custom message along to the ghost actor.
                pub fn ghost_actor_custom(&mut self) -> [< $name CustomSender >] <'_, C> {
                    [< $name CustomSender >] {
                        sender: &mut self.sender,
                        phantom: ::std::marker::PhantomData,
                    }
                }

                /// Shutdown the actor.
                pub async fn ghost_actor_shutdown(&mut self) -> ::std::result::Result<(), $error> {
                    use [< $name Send >];

                    self.sender.ghost_actor_shutdown(()).await
                }
            }
        }
    };

    /*
    // -- "sender" arm writes our sender struct -- //

    ( @inner_sender
        ($($vis:tt)*), $name:ident, $error:ty,
        $( $doc:expr, $req_name:ident, $req_fname:ident, $req_type:ty, $res_type:ty ),*
    ) => {
        paste::item! {
            #[doc = "A cheaply clone-able handle to control a ghost_actor task."]
            #[derive(Clone)]
            $($vis)* struct [< $name Sender >] <C>
            where
                C: $crate::GhostRequestType,
            {
                sender: ::futures::channel::mpsc::Sender< [< __ $name Protocol >] >,
                phantom: ::std::marker::PhantomData<C>,
            }

            impl<C> [< $name Sender >] <C>
            where
                C: $crate::GhostRequestType,
            {
                /// Library users will likely not use this function,
                /// look to the implementation of your actor for a simpler spawn.
                /// GhostActor implementors will use this to spawn handler tasks.
                pub fn ghost_actor_spawn<I, H>(
                    mut handler: H,
                ) -> (Self, $crate::GhostActorDriver)
                where
                    I: $crate::GhostRequestType,
                    H: [< $name Handler >] <C, I>,
                {
                    let (send, mut recv) = ::futures::channel::mpsc::channel(10);

                    let sender = Self {
                        sender: send,
                        phantom: std::marker::PhantomData,
                    };

                    let shutdown = ::std::sync::Arc::new(
                        ::std::sync::RwLock::new(false)
                    );

                    let mut internal_sender: [< $name InternalSender >] <C, I> =
                        [< $name InternalSender >] {
                            sender: Self::clone(&sender),
                            shutdown: shutdown.clone(),
                            phantom_c: ::std::marker::PhantomData,
                            phantom_i: ::std::marker::PhantomData,
                        };

                    use ::futures::{
                        future::FutureExt,
                        stream::StreamExt,
                    };
                    use [< __ $name Protocol >]::*;
                    let driver_fut = async move {
                        while let Some(proto) = recv.next().await {
                            match proto {
                                __GhostActorShutdown(res, span) => {
                                    let _g = span.enter();
                                    *shutdown
                                        .write()
                                        .expect("can acquire shutdown RwLock")
                                        = true;
                                    let _ = res.send((Ok(()), ::tracing::info_span!("shutdown_response")));
                                }
                                __GhostActorCustom(req, res, span) => {
                                    let _g = span.enter();
                                    let req = req.downcast::<C>()
                                        // shouldn't happen -
                                        // we control the incoming types
                                        .expect("bad type sent into custom");
                                    let _ = res.send(match handler.handle_ghost_actor_custom(&mut internal_sender, *req) {
                                        Ok(res) => Ok(Box::new(res)),
                                        Err(e) => Err(e),
                                    });
                                }
                                __GhostActorInternal(req, res, span) => {
                                    let _g = span.enter();
                                    let req = req.downcast::<I>()
                                        // shouldn't happen -
                                        // we control the incoming types
                                        .expect("bad type sent into internal");
                                    let _ = res.send(match handler.handle_ghost_actor_internal(&mut internal_sender, *req) {
                                        Ok(res) => Ok(Box::new(res)),
                                        Err(e) => Err(e),
                                    });
                                }
                                $(
                                    $req_name(req, res, span) => {
                                        let _g = span.enter();
                                        let _ = res.send((
                                            handler. [< handle_ $req_fname >] (&mut internal_sender, req),
                                            ::tracing::info_span!(concat!(stringify!($req_fname), "_response")),
                                        ));
                                    }
                                )*
                            };

                            if *shutdown.read().expect("can acquire shutdown RwLock") {
                                break;
                            }
                        }
                    }.boxed();

                    (
                        sender,
                        driver_fut,
                    )
                }

                $(
                    #[doc = $doc]
                    pub async fn $req_fname (
                        &mut self, input: $req_type,
                    ) -> ::std::result::Result<$res_type, $error> {
                        ::tracing::trace!(request = %stringify!($req_fname));
                        let (send, recv) = ::futures::channel::oneshot::channel();
                        let input = [< __ $name Protocol >] :: $req_name(
                            input, send, ::tracing::info_span!(stringify!($req_fname)));
                        use ::futures::sink::SinkExt;
                        self
                            .sender
                            .send(input)
                            .await
                            .map_err($crate::GhostError::from)?;
                        let (res, span) = recv
                            .await
                            .map_err($crate::GhostError::from)?;
                        let _g = span.enter();
                        ::tracing::trace!(result = ?res);
                        res
                    }
                )*

                /// Shutdown the actor.
                pub async fn ghost_actor_shutdown(&mut self) -> ::std::result::Result<(), $error> {
                    ::tracing::trace!(request = "ghost_actor_shutdown");
                    let (send, recv) = ::futures::channel::oneshot::channel();
                    let input = [< __ $name Protocol >] ::__GhostActorShutdown(
                        send, ::tracing::info_span!("ghost_actor_shutdown"));
                    use ::futures::sink::SinkExt;
                    self
                        .sender
                        .send(input)
                        .await
                        .map_err($crate::GhostError::from)?;
                    let (res, span) = recv
                        .await
                        .map_err($crate::GhostError::from)?;
                    let _g = span.enter();
                    ::tracing::trace!(result = ?res);
                    res
                }

                /// Send a custom message to the actor.
                /// Custom messages give us flexibility in the case of
                /// unanticipated requirements by a particular actor implementation.
                pub async fn ghost_actor_custom(
                    &mut self, input: C,
                ) -> ::std::result::Result<C::ResponseType, $error> {
                    ::tracing::trace!(request = "ghost_actor_custom");
                    let (send, recv) = ::futures::channel::oneshot::channel();
                    let input = [< __ $name Protocol >] ::__GhostActorCustom(
                        Box::new(input), send, ::tracing::info_span!("ghost_actor_custom"));
                    use ::futures::sink::SinkExt;
                    self
                        .sender
                        .send(input)
                        .await
                        .map_err($crate::GhostError::from)?;
                    let res = recv
                        .await
                        .map_err($crate::GhostError::from)?;
                    match res {
                        Ok(res) => {
                            Ok(*res.downcast::<C::ResponseType>()
                                // shouldn't happen -
                                // we control the types
                                .expect("bad response type from custom"))
                        },
                        Err(e) => Err(e),
                    }
                }
            }
        }
    };

    // -- "internal_sender" arm writes our InternalSender struct -- //

    ( @inner_internal_sender
        ($($vis:tt)*), $name:ident, $error:ty,
        $( $doc:expr, $req_name:ident, $req_fname:ident, $req_type:ty, $res_type:ty ),*
    ) => {
        paste::item! {
            #[doc = "The InternalSender accessible from within handlers."]
            $($vis)* struct [< $name InternalSender >] <C, I>
            where
                C: $crate::GhostRequestType,
                I: $crate::GhostRequestType,
            {
                sender: [< $name Sender >]<C>,
                shutdown: ::std::sync::Arc<::std::sync::RwLock<bool>>,
                phantom_c: ::std::marker::PhantomData<C>,
                phantom_i: ::std::marker::PhantomData<I>,
            }

            impl<C, I> ::std::ops::Deref for [< $name InternalSender >] <C, I>
            where
                C: $crate::GhostRequestType,
                I: $crate::GhostRequestType,
            {
                type Target = [< $name Sender >] <C>;

                fn deref(&self) -> &Self::Target {
                    &self.sender
                }
            }

            impl<C, I> ::std::ops::DerefMut for [< $name InternalSender >] <C, I>
            where
                C: $crate::GhostRequestType,
                I: $crate::GhostRequestType,
            {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.sender
                }
            }

            impl<C, I> [< $name InternalSender >] <C, I>
            where
                C: $crate::GhostRequestType,
                I: $crate::GhostRequestType,
            {
                /// Allows a handler to trigger shutdown of the actor task.
                /// All outstanding senders will receive cancel errors.
                pub fn shutdown(&mut self) {
                    *self.shutdown
                        .write()
                        .expect("can acquire shutdown RwLock")
                        = true;
                }

                /// ADVANCED - Sends an internal message to the task
                /// from a handler. Note, this allows you to safely perform
                /// async work without blocking the task handler loop.
                /// However, you'll need to consider how to actually drive
                /// the future returned from this function.
                pub async fn ghost_actor_internal(
                    &mut self, input: I,
                ) -> ::std::result::Result<I::ResponseType, $error> {
                    let (send, recv) = ::futures::channel::oneshot::channel();
                    let input = [< __ $name Protocol >] ::__GhostActorInternal(
                        Box::new(input), send, ::tracing::info_span!("ghost_actor_shutdown"));
                    use ::futures::sink::SinkExt;
                    self
                        .sender
                        .sender
                        .send(input)
                        .await
                        .map_err($crate::GhostError::from)?;
                    let res = recv
                        .await
                        .map_err($crate::GhostError::from)?;
                    match res {
                        Ok(res) => {
                            Ok(*res.downcast::<I::ResponseType>()
                                // shouldn't happen -
                                // we control the types
                                .expect("bad response type from custom"))
                        },
                        Err(e) => Err(e),
                    }
                }
            }
        }
    };
    */
}
