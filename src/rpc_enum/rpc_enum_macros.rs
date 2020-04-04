/// RpcEnum provides a basis for constructing RpcChannels and eventually
/// GhostActors. RpcEnum provides differentiated constructor functions,
/// that generate appropriate input and async await output types.
#[macro_export]
macro_rules! rpc_enum {
    // using @inner_ self references so we don't have to export / pollute
    // a bunch of sub macros.

    // -- public api arms -- //

    (
        name: $name:ident,
        error: $error:ty,
        api: { $( $req_name:ident :: $req_fname:ident ( $doc:expr, $req_type:ty, $res_type:ty ) ),* }
    ) => {
        $crate::rpc_enum! { @inner
            (), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type ),*
        }
    };

    (
        name: $name:ident,
        error: $error:ty,
        api: { $( $req_name:ident :: $req_fname:ident ( $doc:expr, $req_type:ty, $res_type:ty ) ),*, }
    ) => {
        $crate::rpc_enum! { @inner
            (), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type ),*
        }
    };

    (
        name: pub $name:ident,
        error: $error:ty,
        api: { $( $req_name:ident :: $req_fname:ident ( $doc:expr, $req_type:ty, $res_type:ty ) ),*, }
    ) => {
        $crate::rpc_enum! { @inner
            (pub), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type ),*
        }
    };

    (
        name: pub $name:ident,
        error: $error:ty,
        api: { $( $req_name:ident :: $req_fname:ident ( $doc:expr, $req_type:ty, $res_type:ty ) ),* }
    ) => {
        $crate::rpc_enum! { @inner
            (pub), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type ),*
        }
    };

    // -- "inner" arm dispatches to further individual inner arm helpers -- //

    ( @inner
        ($($vis:tt)*), $name:ident, $error:ty,
        $( $doc:expr, $req_name:ident, $req_fname:ident, $req_type:ty, $res_type:ty ),*
    ) => {
        $crate::rpc_enum! { @inner_protocol
            ($($vis)*), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type ),*
        }
        $crate::rpc_enum! { @inner_protocol_fns
            ($($vis)*), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type ),*
        }
    };

    // -- "protocol" arm writes our protocol enum -- //

    ( @inner_protocol
        ($($vis:tt)*), $name:ident, $error:ty,
        $( $doc:expr, $req_name:ident, $req_fname:ident, $req_type:ty, $res_type:ty ),*
    ) => {
        #[derive(Debug)]
        #[doc = "RpcEnum protocol enum."]
        $($vis)* enum $name {
            $(
                $req_name (RpcEnumType<
                    $req_type,
                    ::std::result::Result<$res_type, $error>,
                >),
            )*
        }
    };

    // -- "protocol_fns" arm writes our protocol enum request functions -- //

    ( @inner_protocol_fns
        ($($vis:tt)*), $name:ident, $error:ty,
        $( $doc:expr, $req_name:ident, $req_fname:ident, $req_type:ty, $res_type:ty ),*
    ) => {
        impl $name {
            $(
                pub fn $req_fname (input: $req_type) -> (
                    Self,
                    ::futures::channel::oneshot::Receiver<
                        ::std::result::Result<$res_type, $error>,
                    >,
                ) {
                    let (send, recv) = ::futures::channel::oneshot::channel();
                    let t: RpcEnumType<
                        $req_type,
                        ::std::result::Result<$res_type, $error>,
                    > = RpcEnumType {
                        input,
                        respond: Box::new(move |res: ::std::result::Result<$res_type, $error>| {
                            send
                                .send(res)
                                .map_err(|_|RpcEnumError::from("send failed"))?;
                            Ok(())
                        }),
                        span: tracing::debug_span!(stringify!($req_fname)),
                    };
                    (
                        $name :: $req_name ( t ),
                        recv,
                    )
                }
            )*
        }
    };
}
