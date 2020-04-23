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
        api: { $( $req_name:ident :: $req_fname:ident ( $doc:expr, $req_type:ty, $res_type:ty ) ),* }
    ) => {
        $crate::ghost_actor! { @inner
            (pub), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type ),*
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

    // -- "inner" arm dispatches to further individual inner arm helpers -- //

    ( @inner
        ($($vis:tt)*), $name:ident, $error:ty,
        $( $doc:expr, $req_name:ident, $req_fname:ident, $req_type:ty, $res_type:ty ),*
    ) => {
        $crate::ghost_actor! { @inner_types
            ($($vis)*), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type ),*
        }
        $crate::dependencies::paste::item! {
            $crate::ghost_actor! { @inner2
                ($($vis)*), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type, [< $name Future >] <$res_type> ),*
            }
        }
    };

    // -- "inner2" has some slight type alterations -- //

    ( @inner2
        ($($vis:tt)*), $name:ident, $error:ty,
        $( $doc:expr, $req_name:ident, $req_fname:ident, $req_type:ty, $res_type:ty, $res_type2:ty ),*
    ) => {
        $crate::ghost_chan! { @inner
            (/* not pub */), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type2 ),*,
            "custom", GhostActorCustom, ghost_actor_custom, Box<dyn ::std::any::Any + 'static + Send>, (),
            "internal", GhostActorInternal, ghost_actor_internal, Box<dyn ::std::any::Any + 'static + Send>, (),
            "shutdown", GhostActorShutdown, ghost_actor_shutdown, (), ()
        }
        $crate::ghost_actor! { @inner_handler
            ($($vis)*), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type, $res_type2 ),*
        }
        $crate::ghost_actor! { @inner_sender
            ($($vis)*), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type, $res_type2 ),*
        }
        $crate::ghost_actor! { @inner_internal_sender
            ($($vis)*), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type, $res_type2 ),*
        }
    };

    // -- "types" arm writes our helper typedefs -- //

    ( @inner_types
        ($($vis:tt)*), $name:ident, $error:ty,
        $( $doc:expr, $req_name:ident, $req_fname:ident, $req_type:ty, $res_type:ty ),*
    ) => {
        $crate::dependencies::paste::item! {
            /// Result Type.
            $($vis)* type [< $name Result >] <T> = ::std::result::Result<T, $error>;

            /// Future Type.
            $($vis)* type [< $name Future >] <T> = ::must_future::MustBoxFuture<'static, [< $name Result >] <T> >;

            /// Handler Result Type.
            $($vis)* type [< $name HandlerResult >] <T> = ::std::result::Result<[< $name Future >] <T>, $error>;
        }
    };

    // -- helpers for writing the handler trait functions -- //

    ( @inner_helper_handler $name:ident, $doc:expr, $req_fname:ident, (), $res_type:ty ) => {
        $crate::dependencies::paste::item! {
            #[doc = $doc]
            fn [< handle_ $req_fname >] (
                &mut self
            ) -> [< $name HandlerResult >] <$res_type>;
        }
    };

    ( @inner_helper_handler $name:ident, $doc:expr, $req_fname:ident, $req_type:ty, $res_type:ty ) => {
        $crate::dependencies::paste::item! {
            #[doc = $doc]
            fn [< handle_ $req_fname >] (
                &mut self, input: $req_type,
            ) -> [< $name HandlerResult >] <$res_type>;
        }
    };

    // -- "handler" arm writes our handler trait -- //

    ( @inner_handler
        ($($vis:tt)*), $name:ident, $error:ty,
        $( $doc:expr, $req_name:ident, $req_fname:ident, $req_type:ty, $res_type:ty, $res_type2:ty ),*
    ) => {
        $crate::dependencies::paste::item! {
            #[doc = "Implement this trait to process incoming actor messages."]
            $($vis)* trait [< $name Handler >] <
                C: 'static + Send,
                I: 'static + Send,
            > : 'static + Send {
                // -- api handlers -- //

                $(
                    $crate::ghost_actor! { @inner_helper_handler
                        $name, $doc, $req_fname, $req_type, $res_type
                    }
                )*

                // -- provided -- //

                #[allow(unused_variables)]
                #[doc = "Handle custom messages specific to this exact actor implementation. The provided implementation panics with unimplemented!"]
                fn handle_ghost_actor_custom(
                    &mut self, input: C,
                ) {
                    unimplemented!()
                }

                #[allow(unused_variables)]
                #[doc = "Handle internal messages specific to this exact actor implementation. The provided implementation panics with unimplemented!"]
                fn handle_ghost_actor_internal(
                    &mut self, input: I,
                ) {
                    unimplemented!()
                }
            }
        }
    };

    // -- helpers for invoking the handler trait functions -- //

    ( @inner_helper_invoke_handler $handler:ident, $hname:ident, $item:ident, () ) => {
            let $crate::GhostChanItem {
                respond, span, .. } = $item;
            let _g = span.enter();
            let result = $handler.$hname();
            let _ = respond(result);
    };

    ( @inner_helper_invoke_handler $handler:ident, $hname:ident, $item:ident, $req_type:ty ) => {
            let $crate::GhostChanItem {
                input, respond, span } = $item;
            let _g = span.enter();
            let result = $handler.$hname(input);
            let _ = respond(result);
    };

    // -- helpers for writing sender functions -- //

    ( @inner_helper_sender
        $sender:ident, $doc:expr, $req_fname:ident, (), $res_type:ty
    ) => {
        #[doc = $doc]
        pub async fn $req_fname (
            &mut self,
        ) -> $res_type {
            use $sender;

            self.sender. $req_fname () .await?.await
        }
    };

    ( @inner_helper_sender
        $sender:ident, $doc:expr, $req_fname:ident, $req_type:ty, $res_type:ty
    ) => {
        #[doc = $doc]
        pub async fn $req_fname (
            &mut self, input: $req_type,
        ) -> $res_type {
            use $sender;

            self.sender. $req_fname (input) .await?.await
        }
    };

    // -- "sender" arm writes our sender struct -- //

    ( @inner_sender
        ($($vis:tt)*), $name:ident, $error:ty,
        $( $doc:expr, $req_name:ident, $req_fname:ident, $req_type:ty, $res_type:ty, $res_type2:ty ),*
    ) => {
        $crate::dependencies::paste::item! {
            #[doc = "Helper for ghost_actor Sender custom."]
            $($vis)* struct [< $name Helper >] <'lt, C>
            where
                C: 'static + Send,
            {
                sender: &'lt mut $crate::dependencies::futures::channel::mpsc::Sender<$name>,
                is_internal: bool,
                phantom: ::std::marker::PhantomData<C>,
            }

            impl<C> $crate::GhostChanSend<C> for [< $name Helper >] <'_, C>
            where
                C: 'static + Send,
            {
                fn ghost_chan_send(&mut self, item: C) -> $crate::dependencies::must_future::MustBoxFuture<'_, $crate::GhostResult<()>> {
                    use $crate::dependencies::futures::{future::FutureExt, sink::SinkExt};

                    let input: Box<dyn ::std::any::Any + Send> = Box::new(item);

                    // this item (if it is a GhostChan) already encapsulates
                    // the response handling - send dummy no-op respond
                    let item = $crate::GhostChanItem {
                        input,
                        respond: Box::new(|_| Ok(())),
                        span: $crate::dependencies::tracing::trace_span!("noop"),
                    };

                    let send_fut = if self.is_internal {
                        self.sender.send(
                            $name::GhostActorInternal(item)
                        )
                    } else {
                        self.sender.send(
                            $name::GhostActorCustom(item)
                        )
                    };

                    async move {
                        send_fut.await?;
                        Ok(())
                    }
                    .boxed()
                    .into()
                }
            }

            #[doc = "A cheaply clone-able handle to control a ghost_actor task."]
            $($vis)* struct [< $name Sender >] <C>
            where
                C: 'static + Send,
            {
                sender: $crate::dependencies::futures::channel::mpsc::Sender<$name>,
                phantom: ::std::marker::PhantomData<C>,
            }

            // have to manually impl so we don't introduce clone bound on `C`
            impl<C> ::std::clone::Clone for [< $name Sender >] <C>
            where
                C: 'static + Send,
            {
                fn clone(&self) -> Self {
                    Self {
                        sender: self.sender.clone(),
                        phantom: ::std::marker::PhantomData,
                    }
                }
            }

            impl<C> [< $name Sender >] <C>
            where
                C: 'static + Send,
            {
                /// Library users will likely not use this function,
                /// look to the implementation of your actor for a simpler spawn.
                /// GhostActor implementors will use this to spawn handler tasks.
                pub async fn ghost_actor_spawn<I, H>(
                    factory: $crate::GhostActorSpawn<
                        [< $name InternalSender >] <C, I>,
                        H,
                        $error,
                    >,
                ) -> [< $name Result >]<(Self, $crate::GhostActorDriver)>
                where
                    I: 'static + Send,
                    H: [< $name Handler >] <C, I>,
                {
                    let (send, mut recv) = $crate::dependencies::futures::channel::mpsc::channel(10);

                    let sender = Self {
                        sender: send,
                        phantom: std::marker::PhantomData,
                    };

                    let shutdown = ::std::sync::Arc::new(
                        ::std::sync::RwLock::new(false)
                    );

                    let internal_sender: [< $name InternalSender >] <C, I> =
                        [< $name InternalSender >] {
                            sender: Self::clone(&sender),
                            shutdown: shutdown.clone(),
                            phantom_c: ::std::marker::PhantomData,
                            phantom_i: ::std::marker::PhantomData,
                        };

                    let mut handler = factory(internal_sender).await?;

                    use $crate::dependencies::futures::{
                        future::FutureExt,
                        stream::StreamExt,
                    };
                    let driver_fut = async move {
                        while let Some(proto) = recv.next().await {
                            match proto {
                                $name::GhostActorShutdown(item) => {
                                    let $crate::GhostChanItem {
                                        respond, span, .. } = item;
                                    let _g = span.enter();
                                    *shutdown
                                        .write()
                                        .expect("can acquire shutdown RwLock")
                                        = true;
                                    let _ = respond(Ok(()));
                                }
                                $name::GhostActorCustom(item) => {
                                    let $crate::GhostChanItem {
                                        input, .. } = item;
                                    let input = input.downcast::<C>()
                                        // shouldn't happen -
                                        // we control the incoming types
                                        .expect("bad type sent into custom");
                                    handler.handle_ghost_actor_custom(*input);
                                }
                                $name::GhostActorInternal(item) => {
                                    let $crate::GhostChanItem {
                                        input, .. } = item;
                                    let input = input.downcast::<I>()
                                        // shouldn't happen -
                                        // we control the incoming types
                                        .expect("bad type sent into internal");
                                    handler.handle_ghost_actor_internal(*input);
                                }
                                $(
                                    $name::$req_name(item) => {
                                        $crate::ghost_actor! { @inner_helper_invoke_handler
                                            handler, [< handle_ $req_fname >], item, $req_type
                                        }
                                    }
                                )*
                            };

                            if *shutdown.read().expect("can acquire shutdown RwLock") {
                                break;
                            }
                        }
                    }.boxed().into();

                    Ok((
                        sender,
                        driver_fut,
                    ))
                }

                $(
                    $crate::ghost_actor! { @inner_helper_sender
                        [< $name Send >], $doc, $req_fname, $req_type, [< $name Result >] <$res_type>
                    }
                )*

                /// Send a custom message along to the ghost actor.
                pub fn ghost_actor_custom(&mut self) -> [< $name Helper >] <'_, C> {
                    [< $name Helper >] {
                        sender: &mut self.sender,
                        is_internal: false,
                        phantom: ::std::marker::PhantomData,
                    }
                }

                /// Shutdown the actor.
                pub async fn ghost_actor_shutdown(&mut self) -> [< $name Result >] <()> {
                    use [< $name Send >];

                    self.sender.ghost_actor_shutdown().await
                }
            }
        }
    };

    // -- "internal_sender" arm writes our InternalSender struct -- //

    ( @inner_internal_sender
        ($($vis:tt)*), $name:ident, $error:ty,
        $( $doc:expr, $req_name:ident, $req_fname:ident, $req_type:ty, $res_type:ty, $res_type2:ty ),*
    ) => {
        $crate::dependencies::paste::item! {
            #[doc = "The InternalSender accessible from within handlers."]
            $($vis)* struct [< $name InternalSender >] <C, I>
            where
                C: 'static + Send,
                I: 'static + Send,
            {
                sender: [< $name Sender >]<C>,
                shutdown: ::std::sync::Arc<::std::sync::RwLock<bool>>,
                phantom_c: ::std::marker::PhantomData<C>,
                phantom_i: ::std::marker::PhantomData<I>,
            }

            // have to manually impl so we don't introduce clone bound on `C`, `I`
            impl<C, I> ::std::clone::Clone for [< $name InternalSender >] <C, I>
            where
                C: 'static + Send,
                I: 'static + Send,
            {
                fn clone(&self) -> Self {
                    Self {
                        sender: self.sender.clone(),
                        shutdown: self.shutdown.clone(),
                        phantom_c: ::std::marker::PhantomData,
                        phantom_i: ::std::marker::PhantomData,
                    }
                }
            }

            impl<C, I> ::std::ops::Deref for [< $name InternalSender >] <C, I>
            where
                C: 'static + Send,
                I: 'static + Send,
            {
                type Target = [< $name Sender >] <C>;

                fn deref(&self) -> &Self::Target {
                    &self.sender
                }
            }

            impl<C, I> ::std::ops::DerefMut for [< $name InternalSender >] <C, I>
            where
                C: 'static + Send,
                I: 'static + Send,
            {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.sender
                }
            }

            impl<C, I> [< $name InternalSender >] <C, I>
            where
                C: 'static + Send,
                I: 'static + Send,
            {
                /// Send an internal message back to our handler.
                pub fn ghost_actor_internal(&mut self) -> [< $name Helper >] <'_, I> {
                    [< $name Helper >] {
                        sender: &mut self.sender.sender,
                        is_internal: true,
                        phantom: ::std::marker::PhantomData,
                    }
                }

                /// Allows a handler to trigger shutdown of the actor task.
                /// All outstanding senders will receive cancel errors.
                /// Unlike `ghost_actor_shutdown()`, this call will cancel
                /// the actor task loop immediately.
                pub fn ghost_actor_shutdown_immediate(&mut self) {
                    *self.shutdown
                        .write()
                        .expect("can acquire shutdown RwLock")
                        = true;
                }
            }
        }
    };
}
