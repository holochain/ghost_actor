/// Call `ghost_actor!` to generate the boilerplate for GhostActor implementations.
#[macro_export]
macro_rules! ghost_actor {
    // using @inner_ self references so we don't have to export / pollute
    // a bunch of sub macros.

    // -- public api arms -- //

    (
        Doc($tdoc:expr),
        Visibility($($vis:tt)*),
        Name($name:ident),
        Error($error:ty),
        Api { $( $req_name:ident ( $doc:expr, $req_type:ty, $res_type:ty, ) ),*, }
    ) => {
        $crate::ghost_actor! { @inner
            $tdoc, ($($vis)*), $name, $error, $( $doc, $req_name, $req_type, $res_type ),*
        }
    };

    // -- "inner" arm dispatches to further individual inner arm helpers -- //

    ( @inner
        $tdoc:expr, ($($vis:tt)*), $name:ident, $error:ty,
        $( $doc:expr, $req_name:ident, $req_type:ty, $res_type:ty ),*
    ) => {
        $crate::dependencies::paste::item! {
            $crate::ghost_actor! { @inner_types
                $tdoc, ($($vis)*), $name, $error, $( $doc, $req_name, [< $req_name:snake >], $req_type, $res_type ),*
            }
            $crate::ghost_actor! { @inner2
                $tdoc, ($($vis)*), $name, $error, $( $doc, $req_name, [< $req_name:snake >], $req_type, $res_type, [< $name Future >] <$res_type> ),*
            }
        }
    };

    // -- "inner2" has some slight type alterations -- //

    ( @inner2
        $tdoc:expr, ($($vis:tt)*), $name:ident, $error:ty,
        $( $doc:expr, $req_name:ident, $req_fname:ident, $req_type:ty, $res_type:ty, $res_type2:ty ),*
    ) => {
        $crate::ghost_chan! { @inner
            "internal private channel", (/* not pub */), $name, $error, $( $doc, $req_name, $req_type, $res_type2 ),*,
            "custom", GhostActorCustom, Box<dyn ::std::any::Any + 'static + Send>, (),
            "internal", GhostActorInternal, Box<dyn ::std::any::Any + 'static + Send>, (),
            "shutdown", GhostActorShutdown, (), ()
        }
        $crate::ghost_actor! { @inner_handler
            $tdoc, ($($vis)*), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type, $res_type2 ),*
        }
        $crate::ghost_actor! { @inner_sender
            $tdoc, ($($vis)*), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type, $res_type2 ),*
        }
        $crate::ghost_actor! { @inner_internal_sender
            $tdoc, ($($vis)*), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type, $res_type2 ),*
        }
    };

    // -- "types" arm writes our helper typedefs -- //

    ( @inner_types
        $tdoc:expr, ($($vis:tt)*), $name:ident, $error:ty,
        $( $doc:expr, $req_name:ident, $req_fname:ident, $req_type:ty, $res_type:ty ),*
    ) => {
        $crate::dependencies::paste::item! {
            /// Result Type.
            $($vis)* type [< $name Result >] <T> = ::std::result::Result<T, $error>;

            /// Future Type.
            $($vis)* type [< $name Future >] <T> = $crate::dependencies::must_future::MustBoxFuture<'static, [< $name Result >] <T> >;

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
        $tdoc:expr, ($($vis:tt)*), $name:ident, $error:ty,
        $( $doc:expr, $req_name:ident, $req_fname:ident, $req_type:ty, $res_type:ty, $res_type2:ty ),*
    ) => {
        $crate::dependencies::paste::item! {
            #[doc = $tdoc]
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
                ) -> [< $name Result >] <()> {
                    unimplemented!()
                }

                #[allow(unused_variables)]
                #[doc = "Handle internal messages specific to this exact actor implementation. The provided implementation panics with unimplemented!"]
                fn handle_ghost_actor_internal(
                    &mut self, input: I,
                ) -> [< $name Result >] <()> {
                    unimplemented!()
                }
            }
        }
    };

    // -- helpers for invoking the handler trait functions -- //

    ( @inner_helper_invoke_handler $handler:ident, $hname:ident, $item:ident, () ) => {
            let $crate::ghost_chan::GhostChanItem {
                respond, span, .. } = $item;
            let _g = span.enter();
            let result = $handler.$hname();
            let _ = respond(result);
    };

    ( @inner_helper_invoke_handler $handler:ident, $hname:ident, $item:ident, $req_type:ty ) => {
            let $crate::ghost_chan::GhostChanItem {
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
        $tdoc:expr, ($($vis:tt)*), $name:ident, $error:ty,
        $( $doc:expr, $req_name:ident, $req_fname:ident, $req_type:ty, $res_type:ty, $res_type2:ty ),*
    ) => {
        $crate::dependencies::paste::item! {
            #[doc = "ghost_actor_custom and ghost_actor_internal use this type to expose senders."]
            $($vis)* struct [< $name Helper >] <'lt, C>
            where
                C: 'static + Send,
            {
                sender: &'lt mut $crate::dependencies::futures::channel::mpsc::Sender<$name>,
                is_internal: bool,
                phantom: ::std::marker::PhantomData<C>,
            }

            impl<C> $crate::ghost_chan::GhostChanSend<C> for [< $name Helper >] <'_, C>
            where
                C: 'static + Send,
            {
                fn ghost_chan_send(&mut self, item: C) -> $crate::dependencies::must_future::MustBoxFuture<'_, $crate::GhostResult<()>> {
                    use $crate::dependencies::futures::future::FutureExt;

                    let input: Box<dyn ::std::any::Any + Send> = Box::new(item);

                    let send_fut = if self.is_internal {
                        self.sender.ghost_actor_internal(input)
                    } else {
                        self.sender.ghost_actor_custom(input)
                    };

                    async move {
                        send_fut.await.map_err(|e| {
                            $crate::GhostError::Other(format!("{:?}", e))
                        })?;
                        Ok(())
                    }
                    .boxed()
                    .into()
                }
            }

            #[doc = $tdoc]
            #[derive(Clone)]
            $($vis)* struct [< $name Sender >]
            {
                sender: $crate::dependencies::futures::channel::mpsc::Sender<$name>,
            }

            impl [< $name Sender >]
            {
                /// Library users will likely not use this function,
                /// look to the implementation of your actor for a simpler spawn.
                /// GhostActor implementors will use this to spawn handler tasks.
                pub async fn ghost_actor_spawn<C, I, H>(
                    factory: $crate::GhostActorSpawn<
                        [< $name InternalSender >] <I>,
                        H,
                        $error,
                    >,
                ) -> [< $name Result >]<(Self, $crate::GhostActorDriver)>
                where
                    C: 'static + Send,
                    I: 'static + Send,
                    H: [< $name Handler >] <C, I>,
                {
                    let (send, mut recv) = $crate::dependencies::futures::channel::mpsc::channel(10);

                    let sender = Self {
                        sender: send,
                    };

                    let shutdown = ::std::sync::Arc::new(
                        ::std::sync::RwLock::new(false)
                    );

                    let internal_sender: [< $name InternalSender >] <I> =
                        [< $name InternalSender >] {
                            sender: Self::clone(&sender),
                            shutdown: shutdown.clone(),
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
                                    let $crate::ghost_chan::GhostChanItem {
                                        respond, span, .. } = item;
                                    let _g = span.enter();
                                    *shutdown
                                        .write()
                                        .expect("can acquire shutdown RwLock")
                                        = true;
                                    let _ = respond(Ok(()));
                                }
                                $name::GhostActorCustom(item) => {
                                    let $crate::ghost_chan::GhostChanItem {
                                        input, respond, span } = item;
                                    let _g = span.enter();
                                    match input.downcast::<C>() {
                                        Ok(input) => {
                                            let result = handler.handle_ghost_actor_custom(*input);
                                            let _ = respond(result);
                                        }
                                        Err(_) => {
                                            let _ = respond(Err($crate::GhostError::InvalidCustomType.into()));
                                            return;
                                        }
                                    }
                                }
                                $name::GhostActorInternal(item) => {
                                    let $crate::ghost_chan::GhostChanItem {
                                        input, respond, span } = item;
                                    let _g = span.enter();
                                    let input = input.downcast::<I>()
                                        // shouldn't happen -
                                        // we control the incoming types
                                        .expect("bad type sent into internal");
                                    let result = handler.handle_ghost_actor_internal(*input);
                                    let _ = respond(result);
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
                pub fn ghost_actor_custom<C>(&mut self) -> [< $name Helper >] <'_, C>
                where
                    C: 'static + Send
                {
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
        $tdoc:expr, ($($vis:tt)*), $name:ident, $error:ty,
        $( $doc:expr, $req_name:ident, $req_fname:ident, $req_type:ty, $res_type:ty, $res_type2:ty ),*
    ) => {
        $crate::dependencies::paste::item! {
            #[doc = $tdoc]
            $($vis)* struct [< $name InternalSender >] <I>
            where
                I: 'static + Send,
            {
                sender: [< $name Sender >],
                shutdown: ::std::sync::Arc<::std::sync::RwLock<bool>>,
                phantom_i: ::std::marker::PhantomData<I>,
            }

            // have to manually impl so we don't introduce clone bound on `I`
            impl<I> ::std::clone::Clone for [< $name InternalSender >] <I>
            where
                I: 'static + Send,
            {
                fn clone(&self) -> Self {
                    Self {
                        sender: self.sender.clone(),
                        shutdown: self.shutdown.clone(),
                        phantom_i: ::std::marker::PhantomData,
                    }
                }
            }

            impl<I> ::std::ops::Deref for [< $name InternalSender >] <I>
            where
                I: 'static + Send,
            {
                type Target = [< $name Sender >];

                fn deref(&self) -> &Self::Target {
                    &self.sender
                }
            }

            impl<I> ::std::ops::DerefMut for [< $name InternalSender >] <I>
            where
                I: 'static + Send,
            {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.sender
                }
            }

            impl<I> [< $name InternalSender >] <I>
            where
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
