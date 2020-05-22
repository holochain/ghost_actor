/// Call `ghost_actor!` to generate the boilerplate for GhostActor implementations.
#[macro_export]
macro_rules! ghost_actor_new {
    // using @inner_ self references so we don't have to export / pollute
    // a bunch of sub macros.

    (   @inner_tx
        $(#[$ameta:meta])*
        ($($avis:tt)*) actor $aname:ident<$aerr:ty> {
            $(
                $(#[$rmeta:meta])* fn $rname:ident ( $($pname:ident: $pty:ty),* $(,)? ) -> $rret:ty;
            )*
        }
    ) => {
        $crate::dependencies::paste::item! {
            $crate::ghost_actor_new! { @inner
                $($ameta)* ($($avis)*) $aname $aerr [$(
                    ($($rmeta)*) $rname [< $rname:camel >] $rret [$(
                        $pname $pty
                    )*]
                )*]
            }
        }
    };
    (   @inner
        $($ameta:meta)* ($($avis:tt)*) $aname:ident $aerr:ty [$(
            ($($rmeta:meta)*) $rname:ident $rnamec:ident $rret:ty [$(
                $pname:ident $pty:ty
            )*]
        )*]
    ) => {
        $crate::dependencies::paste::item! {
            $crate::ghost_chan_new! { @inner
                $($ameta)* (/* not pub */ pub) $aname $aerr [
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
            $crate::ghost_actor_new! { @inner_types
                $($ameta)* ($($avis)*) $aname $aerr [$(
                    ($($rmeta)*) $rname $rnamec $rret [$(
                        $pname $pty
                    )*]
                )*]
            }
            $crate::ghost_actor_new! { @inner_handler
                $($ameta)* ($($avis)*) $aname $aerr [$(
                    ($($rmeta)*) $rname $rnamec $rret [$(
                        $pname $pty
                    )*]
                )*]
            }
            $crate::ghost_actor_new! { @inner_sender
                $($ameta)* ($($avis)*) $aname $aerr [$(
                    ($($rmeta)*) $rname $rnamec $rret [$(
                        $pname $pty
                    )*]
                )*]
            }
            $crate::ghost_actor_new! { @inner_internal_sender
                $($ameta)* ($($avis)*) $aname $aerr [$(
                    ($($rmeta)*) $rname $rnamec $rret [$(
                        $pname $pty
                    )*]
                )*]
            }
        }
    };
    (   @inner_types
        $($ameta:meta)* ($($avis:tt)*) $aname:ident $aerr:ty [$(
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
    (   @inner_handler
        $($ameta:meta)* ($($avis:tt)*) $aname:ident $aerr:ty [$(
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
    (   @inner_sender
        $($ameta:meta)* ($($avis:tt)*) $aname:ident $aerr:ty [$(
            ($($rmeta:meta)*) $rname:ident $rnamec:ident $rret:ty [$(
                $pname:ident $pty:ty
            )*]
        )*]
    ) => {
        $crate::dependencies::paste::item! {
            #[doc = "ghost_actor_custom and ghost_actor_internal use this type to expose senders."]
            $($avis)* struct [< $aname Helper >] <'lt, C>
            where
                C: 'static + Send,
            {
                sender: &'lt mut $crate::dependencies::futures::channel::mpsc::Sender<$aname>,
                is_internal: bool,
                phantom: ::std::marker::PhantomData<C>,
            }

            impl<C> $crate::ghost_chan::GhostChanSend<C> for [< $aname Helper >] <'_, C>
            where
                C: 'static + Send,
            {
                fn ghost_chan_send(&mut self, item: C) -> $crate::dependencies::must_future::MustBoxFuture<'_, $crate::GhostResult<()>> {
                    use $crate::dependencies::futures::future::FutureExt;

                    let input: Box<dyn ::std::any::Any + Send> = Box::new(item);

                    let send_fut = if self.is_internal {
                        [< $aname Send >]::ghost_actor_internal(self.sender, input)
                    } else {
                        [< $aname Send >]::ghost_actor_custom(self.sender, input)
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

            $(#[$ameta])*
            #[derive(Clone)]
            $($avis)* struct [< $aname Sender >] {
                sender: $crate::dependencies::futures::channel::mpsc::Sender<$aname>,
            }

            impl [< $aname Sender >] {
                /// Library users will likely not use this function,
                /// look to the implementation of your actor for a simpler spawn.
                /// GhostActor implementors will use this to spawn handler tasks.
                pub async fn ghost_actor_spawn<C, I, H>(
                    factory: $crate::GhostActorSpawn<
                        [< $aname InternalSender >] <I>,
                        H,
                        $aerr,
                    >,
                ) -> [< $aname Result >]<(Self, $crate::GhostActorDriver)>
                where
                    C: 'static + Send,
                    I: 'static + Send,
                    H: [< $aname Handler >] <C, I>,
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

                    use $crate::dependencies::futures::{
                        future::FutureExt,
                        stream::StreamExt,
                    };
                    let driver_fut = async move {
                        while let Some(proto) = recv.next().await {
                            match proto {
                                $aname::GhostActorShutdown(item) => {
                                    let $crate::ghost_chan::GhostChanItem {
                                        respond, span, .. } = item;
                                    let _g = span.enter();
                                    *shutdown
                                        .write()
                                        .expect("can acquire shutdown RwLock")
                                        = true;
                                    let _ = respond(Ok(()));
                                }
                                $aname::GhostActorCustom(item) => {
                                    let $crate::ghost_chan::GhostChanItem {
                                        input, respond, span } = item;
                                    let _g = span.enter();
                                    match input.0.downcast::<C>() {
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
                                $aname::GhostActorInternal(item) => {
                                    let $crate::ghost_chan::GhostChanItem {
                                        input, respond, span } = item;
                                    let _g = span.enter();
                                    let input = input.0.downcast::<I>()
                                        // shouldn't happen -
                                        // we control the incoming types
                                        .expect("bad type sent into internal");
                                    let result = handler.handle_ghost_actor_internal(*input);
                                    let _ = respond(result);
                                }
                                $(
                                    $aname::$rnamec(item) => {
                                        let $crate::ghost_chan::GhostChanItem {
                                            input, respond, span } = item;
                                        let _g = span.enter();
                                        let ($($pname,)*) = input;
                                        let result = handler.[< handle_ $rname >](
                                            $($pname,)*
                                        );
                                        let _ = respond(result);
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
                    $(#[$rmeta])*
                    pub async fn $rname (
                        &mut self, $($pname: $pty,)*
                    ) -> [< $aname Result >] <$rret> {
                        [< $aname Send >]::$rname(
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
                    [< $aname Send >]::ghost_actor_shutdown(&mut self.sender).await
                }
            }
        }
    };
    (   @inner_internal_sender
        $($ameta:meta)* ($($avis:tt)*) $aname:ident $aerr:ty [$(
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
    (
        $(#[$ameta:meta])* pub ( $($avis:tt)* ) actor $($rest:tt)*
    ) => {
        $crate::ghost_actor_new! { @inner_tx
            #[$($ameta)*] (pub($($avis)*)) actor $($rest)*
        }
    };
    (
        $(#[$ameta:meta])* pub actor $($rest:tt)*
    ) => {
        $crate::ghost_actor_new! { @inner_tx
            #[$($ameta)*] (pub) actor $($rest)*
        }
    };
    (
        $(#[$ameta:meta])* actor $($rest:tt)*
    ) => {
        $crate::ghost_actor_new! { @inner_tx
            #[$($ameta)*] () actor $($rest)*
        }
    };
}
