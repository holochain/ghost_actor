/// RpcChan provides a basis for constructing RpcChannels and eventually
/// GhostActors. RpcChan provides differentiated constructor functions,
/// that generate appropriate input and async await output types.
#[macro_export]
macro_rules! rpc_chan {
    // using @inner_ self references so we don't have to export / pollute
    // a bunch of sub macros.

    // -- public api arms -- //

    (
        name: $name:ident,
        error: $error:ty,
        api: { $( $req_name:ident :: $req_fname:ident ( $doc:expr, $req_type:ty, $res_type:ty ) ),* }
    ) => {
        $crate::rpc_chan! { @inner
            (), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type ),*
        }
    };

    (
        name: $name:ident,
        error: $error:ty,
        api: { $( $req_name:ident :: $req_fname:ident ( $doc:expr, $req_type:ty, $res_type:ty ) ),*, }
    ) => {
        $crate::rpc_chan! { @inner
            (), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type ),*
        }
    };

    (
        name: pub $name:ident,
        error: $error:ty,
        api: { $( $req_name:ident :: $req_fname:ident ( $doc:expr, $req_type:ty, $res_type:ty ) ),*, }
    ) => {
        $crate::rpc_chan! { @inner
            (pub), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type ),*
        }
    };

    (
        name: pub $name:ident,
        error: $error:ty,
        api: { $( $req_name:ident :: $req_fname:ident ( $doc:expr, $req_type:ty, $res_type:ty ) ),* }
    ) => {
        $crate::rpc_chan! { @inner
            (pub), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type ),*
        }
    };

    // -- "inner" arm dispatches to further individual inner arm helpers -- //

    ( @inner
        ($($vis:tt)*), $name:ident, $error:ty,
        $( $doc:expr, $req_name:ident, $req_fname:ident, $req_type:ty, $res_type:ty ),*
    ) => {
        $crate::rpc_chan! { @inner_protocol
            ($($vis)*), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type ),*
        }
        $crate::rpc_chan! { @inner_send_trait
            ($($vis)*), $name, $error, $( $doc, $req_name, $req_fname, $req_type, $res_type ),*
        }
    };

    // -- "protocol" arm writes our protocol enum -- //

    ( @inner_protocol
        ($($vis:tt)*), $name:ident, $error:ty,
        $( $doc:expr, $req_name:ident, $req_fname:ident, $req_type:ty, $res_type:ty ),*
    ) => {
        #[derive(Debug)]
        #[doc = "RpcChan protocol enum."]
        $($vis)* enum $name {
            $(
                #[doc = $doc]
                $req_name ($crate::RpcChanItem<
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
            #[doc = "RpcChan protocol enum send trait."]
            $($vis)* trait [< $name Send >] {
                /// Implement this in your sender newtype to forward RpcChan messages across a
                /// channel.
                fn rpc_chan_send(&mut self, item: $name) -> ::must_future::MustBoxFuture<'_, $crate::RpcChanResult<()>>;

                $(
                    #[ doc = $doc ]
                    fn $req_fname ( &mut self, input: $req_type ) -> ::must_future::MustBoxFuture<'_, ::std::result::Result<$res_type, $error>> {
                        let (send, recv) = ::futures::channel::oneshot::channel();
                        let t = $crate::RpcChanItem {
                            input,
                            respond: Box::new(move |res| {
                                if let Err(_) = send.send(res) {
                                    return Err($crate::RpcChanError::from("send error"));
                                }
                                Ok(())
                            }),
                            span: tracing::debug_span!(stringify!($req_fname)),
                        };

                        let t = $name :: $req_name ( t );

                        let send_fut = self.rpc_chan_send(t);

                        use ::futures::future::FutureExt;

                        async move {
                            send_fut.await?;
                            recv.await.map_err($crate::RpcChanError::from)?
                        }.boxed().into()
                    }
                )*
            }

            impl [< $name Send >] for ::futures::channel::mpsc::Sender<$name> {
                fn rpc_chan_send(&mut self, item: $name) -> ::must_future::MustBoxFuture<'_, $crate::RpcChanResult<()>> {
                    use ::futures::{
                        future::FutureExt,
                        sink::SinkExt,
                    };

                    let send_fut = self.send(item);

                    async move {
                        send_fut.await?;
                        Ok(())
                    }.boxed().into()
                }
            }
        }
    };
}
