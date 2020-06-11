/// Call `ghost_actor!` to generate the boilerplate for GhostActor implementations.
#[macro_export]
macro_rules! ghost_actor {
    // using @inner_ self references so we don't have to export / pollute
    // a bunch of sub macros.

    // -- inner_tx does some translation from our external macro api
    // -- to a simpler internal api

    (   @inner_tx
        $(#[$ameta:meta])*
        ($($avis:tt)*) actor $aname:ident<$aerr:ty> {
            $(
                $(#[$rmeta:meta])* fn $rname:ident ( $($pname:ident: $pty:ty),* $(,)? ) -> $rret:ty;
            )*
        }
    ) => {
        $crate::dependencies::paste::item! {
            $crate::ghost_actor! { @inner
                ($($ameta)*) ($($avis)*) $aname $aerr [$(
                    ($($rmeta)*) $rname [< $rname:camel >] $rret [$(
                        $pname $pty
                    )*]
                )*]
            }
        }
    };

    // -- the main entrypoint to our internal api
    // -- dispatches to sub functions

    (   @inner
        ($($ameta:meta)*) ($($avis:tt)*) $aname:ident $aerr:ty [$(
            ($($rmeta:meta)*) $rname:ident $rnamec:ident $rret:ty [$(
                $pname:ident $pty:ty
            )*]
        )*]
    ) => {
        $crate::dependencies::paste::item! {
            mod [< __ghost_actor_ $aname:snake _chan >] {
                use super::*;

                $crate::ghost_chan! { @inner
                    ($($ameta)*) (pub(super)) $aname $aerr [
                        $(
                            ($($rmeta)*) $rname $rnamec [< $aname Future >] <$rret> [$(
                                $pname $pty
                            )*]
                        )*

                        (doc = "internal 'custom' request type")
                        ghost_actor_custom GhostActorCustom ()
                        [ input Box<dyn ::std::any::Any + 'static + Send> ]

                        (doc = "internal 'internal' request type")
                        ghost_actor_internal GhostActorInternal ()
                        [ input Box<dyn ::std::any::Any + 'static + Send> ]

                        (doc = "internal 'shutdown' request type")
                        ghost_actor_shutdown GhostActorShutdown ()
                        [ ]
                    ]
                }
            }

            $crate::ghost_actor! { @inner_types
                ($($ameta)*) ($($avis)*) $aname $aerr [$(
                    ($($rmeta)*) $rname $rnamec $rret [$(
                        $pname $pty
                    )*]
                )*]
            }
            $crate::ghost_actor! { @inner_handler
                ($($ameta)*) ($($avis)*) $aname $aerr [$(
                    ($($rmeta)*) $rname $rnamec $rret [$(
                        $pname $pty
                    )*]
                )*]
            }
            $crate::ghost_actor! { @inner_sender
                ($($ameta)*) ($($avis)*) $aname $aerr [$(
                    ($($rmeta)*) $rname $rnamec $rret [$(
                        $pname $pty
                    )*]
                )*]
            }
            $crate::ghost_actor! { @inner_internal_sender
                ($($ameta)*) ($($avis)*) $aname $aerr [$(
                    ($($rmeta)*) $rname $rnamec $rret [$(
                        $pname $pty
                    )*]
                )*]
            }
        }
    };

    // -- some helper type aliases -- //

    (   @inner_types
        ($($ameta:meta)*) ($($avis:tt)*) $aname:ident $aerr:ty [$(
            ($($rmeta:meta)*) $rname:ident $rnamec:ident $rret:ty [$(
                $pname:ident $pty:ty
            )*]
        )*]
    ) => {
        $crate::dependencies::paste::item! {
            /// Result Type
            $($avis)* type [< $aname Result >] <T> = ::std::result::Result<T, $aerr>;

            /// Future Type.
            $($avis)* type [< $aname Future >] <T> = $crate::dependencies::must_future::MustBoxFuture<'static, [< $aname Result >] <T> >;

            /// Handler Result Type.
            $($avis)* type [< $aname HandlerResult >] <T> = ::std::result::Result<[< $aname Future >] <T>, $aerr>;
        }
    };

    // -- write the handler trait for implementing actors of this type -- //

    (   @inner_handler
        ($($ameta:meta)*) ($($avis:tt)*) $aname:ident $aerr:ty [$(
            ($($rmeta:meta)*) $rname:ident $rnamec:ident $rret:ty [$(
                $pname:ident $pty:ty
            )*]
        )*]
    ) => {
        $crate::dependencies::paste::item! {
            $(#[$ameta])*
            $($avis)* trait [< $aname Handler >] <
                C: 'static + Send,
                I: 'static + Send,
            > : 'static + Send {
                // -- api handlers -- //

                $(
                    $(#[$rmeta])*
                    fn [< handle_ $rname >] (
                        &mut self, $($pname: $pty,)*
                    ) -> [< $aname HandlerResult >]<$rret>;
                )*

                // -- provided -- //

                #[doc = "Gives actors a chance to handle any cleanup tasks before the actor is shut down. This provided function is a no-op."]
                fn handle_ghost_actor_shutdown(&mut self) {
                }

                #[allow(unused_variables)]
                #[doc = "Handle custom messages specific to this exact actor implementation. The provided implementation panics with unimplemented!"]
                fn handle_ghost_actor_custom(
                    &mut self, input: C,
                ) -> [< $aname Result >] <()> {
                    unimplemented!()
                }

                #[allow(unused_variables)]
                #[doc = "Handle internal messages specific to this exact actor implementation. The provided implementation panics with unimplemented!"]
                fn handle_ghost_actor_internal(
                    &mut self, input: I,
                ) -> [< $aname Result >] <()> {
                    unimplemented!()
                }
            }
        }
    };

    // -- write the sender that will be used to access actors of this type -- //

    (   @inner_sender
        ($($ameta:meta)*) ($($avis:tt)*) $aname:ident $aerr:ty [$(
            ($($rmeta:meta)*) $rname:ident $rnamec:ident $rret:ty [$(
                $pname:ident $pty:ty
            )*]
        )*]
    ) => {
        $crate::dependencies::paste::item! {
            // this is a helper struct

            #[doc = "ghost_actor_custom and ghost_actor_internal use this type to expose senders."]
            $($avis)* struct [< $aname Helper >] <'lt, C>
            where
                C: 'static + Send,
            {
                sender: &'lt mut $crate::dependencies::futures::channel::mpsc::Sender<[< __ghost_actor_ $aname:snake _chan >]::$aname>,
                is_internal: bool,
                phantom: ::std::marker::PhantomData<C>,
            }

            impl<C> $crate::ghost_chan::GhostChanSend<C> for [< $aname Helper >] <'_, C>
            where
                C: 'static + Send,
            {
                fn ghost_chan_send(&mut self, item: C) -> $crate::dependencies::must_future::MustBoxFuture<'_, $crate::GhostResult<()>> {
                    let input: Box<dyn ::std::any::Any + Send> = Box::new(item);

                    let send_fut = if self.is_internal {
                        [< __ghost_actor_ $aname:snake _chan >]::[< $aname Send >]::ghost_actor_internal(self.sender, input)
                    } else {
                        [< __ghost_actor_ $aname:snake _chan >]::[< $aname Send >]::ghost_actor_custom(self.sender, input)
                    };

                    $crate::dependencies::must_future::MustBoxFuture::new(async move {
                        send_fut.await.map_err(|e| {
                            $crate::GhostError::from(format!("{:?}", e))
                        })?;
                        Ok(())
                    })
                }
            }

            // the actual sender

            $(#[$ameta])*
            #[derive(Clone)]
            $($avis)* struct [< $aname Sender >] {
                sender: $crate::dependencies::futures::channel::mpsc::Sender<[< __ghost_actor_ $aname:snake _chan >]::$aname>,
            }

            impl [< $aname Sender >] {
                /// Library users will likely not use this function,
                /// look to the implementation of your actor for a simpler spawn.
                /// GhostActor implementors will use this to spawn handler tasks.
                pub async fn ghost_actor_spawn<'a, C, I, H, F>(
                    factory: F,
                ) -> [< $aname Result >]<(Self, $crate::GhostActorDriver)>
                where
                    C: 'static + Send,
                    I: 'static + Send,
                    H: [< $aname Handler >] <C, I>,
                    F: 'a + FnOnce([< $aname InternalSender >]<I>) -> $crate::dependencies::must_future::MustBoxFuture<'static, ::std::result::Result<H, $aerr>> + Send,
                {
                    let (send, mut recv) = $crate::dependencies::futures::channel::mpsc::channel(10);

                    let sender = Self {
                        sender: send,
                    };

                    let shutdown = ::std::sync::Arc::new(
                        ::std::sync::RwLock::new(false)
                    );

                    let internal_sender: [< $aname InternalSender >] <I> =
                        [< $aname InternalSender >] {
                            sender: Self::clone(&sender),
                            shutdown: shutdown.clone(),
                            phantom_i: ::std::marker::PhantomData,
                        };

                    let mut handler = factory(internal_sender).await?;

                    let driver_fut = $crate::dependencies::must_future::MustBoxFuture::new(async move {
                        while let Some(proto) = $crate::dependencies::futures::stream::StreamExt::next(&mut recv).await {
                            match proto {
                                [< __ghost_actor_ $aname:snake _chan >]::$aname::GhostActorShutdown { span, respond } => {
                                    let _g = span.enter();
                                    *shutdown
                                        .write()
                                        .expect("can acquire shutdown RwLock")
                                        = true;
                                    respond.respond(Ok(()));
                                }
                                [< __ghost_actor_ $aname:snake _chan >]::$aname::GhostActorCustom { span, respond, input } => {
                                    let _g = span.enter();
                                    match input.downcast::<C>() {
                                        Ok(input) => {
                                            let result = handler.handle_ghost_actor_custom(*input);
                                            respond.respond(result);
                                        }
                                        Err(_) => {
                                            respond.respond(Err($crate::GhostError::InvalidCustomType.into()));
                                            return;
                                        }
                                    }
                                }
                                [< __ghost_actor_ $aname:snake _chan >]::$aname::GhostActorInternal { span, respond, input } => {
                                    let _g = span.enter();
                                    let input = input.downcast::<I>()
                                        // shouldn't happen -
                                        // we control the incoming types
                                        .expect("bad type sent into internal");
                                    let result = handler.handle_ghost_actor_internal(*input);
                                    respond.respond(result);
                                }
                                $(
                                    [< __ghost_actor_ $aname:snake _chan >]::$aname::$rnamec { span, respond, $($pname,)* } => {
                                        let _g = span.enter();
                                        let result = handler.[< handle_ $rname >](
                                            $($pname,)*
                                        );
                                        respond.respond(result);
                                    }
                                )*
                            };

                            if *shutdown.read().expect("can acquire shutdown RwLock") {
                                break;
                            }
                        }
                        handler.handle_ghost_actor_shutdown();
                    });

                    Ok((
                        sender,
                        driver_fut,
                    ))
                }

                $(
                    $(#[$rmeta])*
                    pub async fn $rname (
                        &mut self, $($pname: $pty,)*
                    ) -> [< $aname Result >] <$rret> {
                        [< __ghost_actor_ $aname:snake _chan >]::[< $aname Send >]::$rname(
                            &mut self.sender, $($pname,)*
                        ).await?.await
                    }
                )*

                /// Send a custom message along to the ghost actor.
                pub fn ghost_actor_custom<C>(&mut self) -> [< $aname Helper >] <'_, C>
                where
                    C: 'static + Send
                {
                    [< $aname Helper >] {
                        sender: &mut self.sender,
                        is_internal: false,
                        phantom: ::std::marker::PhantomData,
                    }
                }

                /// Shutdown the actor.
                pub async fn ghost_actor_shutdown(&mut self) -> [< $aname Result >] <()> {
                    [< __ghost_actor_ $aname:snake _chan >]::[< $aname Send >]::ghost_actor_shutdown(&mut self.sender).await
                }
            }
        }
    };

    // -- write the "internal" sender that will have additional functinality -- //

    (   @inner_internal_sender
        ($($ameta:meta)*) ($($avis:tt)*) $aname:ident $aerr:ty [$(
            ($($rmeta:meta)*) $rname:ident $rnamec:ident $rret:ty [$(
                $pname:ident $pty:ty
            )*]
        )*]
    ) => {
        $crate::dependencies::paste::item! {
            /// InternalSender is used when creating an actor implementation.
            $($avis)* struct [< $aname InternalSender >] <I>
            where
                I: 'static + Send,
            {
                sender: [< $aname Sender >],
                shutdown: ::std::sync::Arc<::std::sync::RwLock<bool>>,
                phantom_i: ::std::marker::PhantomData<I>,
            }

            // have to manually impl so we don't introduce clone bound on `I`
            impl<I> ::std::clone::Clone for [< $aname InternalSender >] <I>
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

            impl<I> ::std::ops::Deref for [< $aname InternalSender >] <I>
            where
                I: 'static + Send,
            {
                type Target = [< $aname Sender >];

                fn deref(&self) -> &Self::Target {
                    &self.sender
                }
            }

            impl<I> ::std::ops::DerefMut for [< $aname InternalSender >] <I>
            where
                I: 'static + Send,
            {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.sender
                }
            }

            impl<I> [< $aname InternalSender >] <I>
            where
                I: 'static + Send,
            {
                /// Send an internal message back to our handler.
                pub fn ghost_actor_internal(&mut self) -> [< $aname Helper >] <'_, I> {
                    [< $aname Helper >] {
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

    // -- visibility helpers - these are the arms users actually invoke -- //

    // specialized pub visibility
    (
        $(#[$ameta:meta])* pub ( $($avis:tt)* ) actor $($rest:tt)*
    ) => {
        $crate::ghost_actor! { @inner_tx
            $(#[$ameta])* (pub($($avis)*)) actor $($rest)*
        }
    };

    // generic pub visibility
    (
        $(#[$ameta:meta])* pub actor $($rest:tt)*
    ) => {
        $crate::ghost_actor! { @inner_tx
            $(#[$ameta])* (pub) actor $($rest)*
        }
    };

    // private visibility
    (
        $(#[$ameta:meta])* actor $($rest:tt)*
    ) => {
        $crate::ghost_actor! { @inner_tx
            $(#[$ameta])* () actor $($rest)*
        }
    };
}
