/// The `ghost_chan!` macro generates an enum and helper types that make it
/// easy to make inline async requests and await responses.
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
                $req_name ($crate::ghost_chan::GhostChanItem<
                    $req_type,
                    ::std::result::Result<$res_type, $error>,
                >),
            )*
        }
    };

    // -- helpers for writing out the sender trait functions -- //

    ( @inner_helper_sender
        $sendf:ident, $doc:expr, $req_fname:ident, (), $res_type:ty, $error:ty
    ) => {
        #[ doc = $doc ]
        fn $req_fname ( &mut self ) -> $crate::dependencies::must_future::MustBoxFuture<'_, ::std::result::Result<$res_type, $error>> {
            $sendf(self, ())
        }
    };

    ( @inner_helper_sender
        $sendf:ident, $doc:expr, $req_fname:ident, $req_type:ty, $res_type:ty, $error:ty
    ) => {
        #[ doc = $doc ]
        fn $req_fname ( &mut self, input: $req_type ) -> $crate::dependencies::must_future::MustBoxFuture<'_, ::std::result::Result<$res_type, $error>> {
            $sendf(self, input)
        }
    };

    // -- "send_trait" arm writes our protocol send trait -- //

    ( @inner_send_trait
        ($($vis:tt)*), $name:ident, $error:ty,
        $( $doc:expr, $req_name:ident, $req_fname:ident, $req_type:ty, $res_type:ty ),*
    ) => {
        $crate::dependencies::paste::item! {
            $(
                #[allow(non_snake_case, clippy::needless_lifetimes)]
                fn [< __ghost_chan_ $name _ $req_fname >]<'lt, S>(
                    sender: &'lt mut S,
                    input: $req_type,
                ) -> $crate::dependencies::must_future::MustBoxFuture<'lt, ::std::result::Result<$res_type, $error>>
                where
                    S: $crate::ghost_chan::GhostChanSend<$name> + ?Sized,
                {
                    $crate::dependencies::tracing::trace!(request = ?input);
                    let (send, recv) = $crate::dependencies::futures::channel::oneshot::channel();
                    let t = $crate::ghost_chan::GhostChanItem {
                        input,
                        respond: Box::new(move |res| {
                            if send.send((res, $crate::dependencies::tracing::debug_span!(
                                concat!(stringify!($req_fname), "_respond")
                            ))).is_err() {
                                return Err($crate::GhostError::from("send error"));
                            }
                            Ok(())
                        }),
                        span: $crate::dependencies::tracing::debug_span!(stringify!($req_fname)),
                    };

                    let t = $name :: $req_name ( t );

                    let send_fut = sender.ghost_chan_send(t);

                    use $crate::dependencies::futures::future::FutureExt;

                    async move {
                        send_fut.await?;
                        let (data, span) = recv.await.map_err($crate::GhostError::from)?;
                        let _g = span.enter();
                        $crate::dependencies::tracing::trace!(response = ?data);
                        data
                    }.boxed().into()
                }
            )*

            #[doc = "GhostChan protocol enum send trait."]
            $($vis)* trait [< $name Send >]: $crate::ghost_chan::GhostChanSend<$name> {
                $(
                    $crate::ghost_chan! { @inner_helper_sender
                        [< __ghost_chan_ $name _ $req_fname >],
                        $doc, $req_fname, $req_type, $res_type, $error
                    }
                )*
            }

            impl<T: $crate::ghost_chan::GhostChanSend<$name>> [< $name Send >] for T {}
        }
    };
}
