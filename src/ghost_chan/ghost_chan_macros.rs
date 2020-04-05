/// GhostChan provides a basis for constructing GhostChannels and eventually
/// GhostActors. GhostChan provides differentiated constructor functions,
/// that generate appropriate input and async await output types.
#[macro_export]
macro_rules! ghost_chan {
    // using @inner_ self references so we don't have to export / pollute
    // a bunch of sub macros.

    // -- public api arms -- //

    (
        name: $name:ident,
        error: $error:ty,
        api: { $( $req_name:ident :: $req_fname:ident ( $doc:expr, $req_type:ty, $res_type:ty ) ),* }
    ) => {
        $crate::ghost_chan! { @inner
            (), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type ),*
        }
    };

    (
        name: $name:ident,
        error: $error:ty,
        api: { $( $req_name:ident :: $req_fname:ident ( $doc:expr, $req_type:ty, $res_type:ty ) ),*, }
    ) => {
        $crate::ghost_chan! { @inner
            (), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type ),*
        }
    };

    (
        name: pub $name:ident,
        error: $error:ty,
        api: { $( $req_name:ident :: $req_fname:ident ( $doc:expr, $req_type:ty, $res_type:ty ) ),*, }
    ) => {
        $crate::ghost_chan! { @inner
            (pub), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type ),*
        }
    };

    (
        name: pub $name:ident,
        error: $error:ty,
        api: { $( $req_name:ident :: $req_fname:ident ( $doc:expr, $req_type:ty, $res_type:ty ) ),* }
    ) => {
        $crate::ghost_chan! { @inner
            (pub), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type ),*
        }
    };

    // -- "inner" arm dispatches to further individual inner arm helpers -- //

    ( @inner
        ($($vis:tt)*), $name:ident, $error:ty,
        $( $doc:expr, $req_name:ident, $req_fname:ident, $req_type:ty, $res_type:ty ),*
    ) => {
        $crate::ghost_chan! { @inner_protocol
            ($($vis)*), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type ),*
        }
        $crate::ghost_chan! { @inner_send_trait
            ($($vis)*), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type ),*
        }
    };

    // -- "protocol" arm writes our protocol enum -- //

    ( @inner_protocol
        ($($vis:tt)*), $name:ident, $error:ty,
        $( $doc:expr, $req_name:ident, $req_fname:ident, $req_type:ty, $res_type:ty ),*
    ) => {
        #[derive(Debug)]
        #[doc = "GhostChan protocol enum."]
        $($vis)* enum $name {
            $(
                #[doc = $doc]
                $req_name ($crate::GhostChanItem<
                    $req_type,
                    ::std::result::Result<$res_type, $error>,
                >),
            )*
        }
    };

    // -- "send_trait" arm writes our protocol send trait -- //

    ( @inner_send_trait
        ($($vis:tt)*), $name:ident, $error:ty,
        $( $doc:expr, $req_name:ident, $req_fname:ident, $req_type:ty, $res_type:ty ),*
    ) => {
        paste::item! {
            #[doc = "GhostChan protocol enum send trait."]
            $($vis)* trait [< $name Send >]: $crate::GhostChanSend<$name> {
                $(
                    #[ doc = $doc ]
                    fn $req_fname ( &mut self, input: $req_type ) -> ::must_future::MustBoxFuture<'_, ::std::result::Result<$res_type, $error>> {
                        tracing::trace!(request = ?input);
                        let (send, recv) = ::futures::channel::oneshot::channel();
                        let t = $crate::GhostChanItem {
                            input,
                            respond: Box::new(move |res| {
                                if let Err(_) = send.send((res, tracing::debug_span!(
                                    concat!(stringify!($req_fname), "_respond")
                                ))) {
                                    return Err($crate::GhostError::from("send error"));
                                }
                                Ok(())
                            }),
                            span: tracing::debug_span!(stringify!($req_fname)),
                        };

                        let t = $name :: $req_name ( t );

                        let send_fut = self.ghost_chan_send(t);

                        use ::futures::future::FutureExt;

                        async move {
                            send_fut.await?;
                            let (data, span) = recv.await.map_err($crate::GhostError::from)?;
                            let _g = span.enter();
                            tracing::trace!(response = ?data);
                            data
                        }.boxed().into()
                    }
                )*
            }

            impl<T: $crate::GhostChanSend<$name>> [< $name Send >] for T {}
        }
    };
}
