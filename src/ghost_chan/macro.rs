/// The `ghost_chan!` macro generates an enum and helper types that make it
/// easy to make inline async requests and await responses.
#[macro_export]
macro_rules! ghost_chan_new {
    // using @inner_ self references so we don't have to export / pollute
    // a bunch of sub macros.

    (   @inner_tx
        $(#[$ameta:meta])*
        ($($avis:tt)*) chan $aname:ident<$aerr:ty> {
            $(
                $(#[$rmeta:meta])* fn $rname:ident ( $($pname:ident: $pty:ty),* $(,)? ) -> $rret:ty;
            )*
        }
    ) => {
        $crate::dependencies::paste::item! {
            $crate::ghost_chan_new! { @inner
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
        $crate::ghost_chan_new! { @inner_protocol
            $($ameta)* ($($avis)*) $aname $aerr [$(
                ($($rmeta)*) $rname $rnamec $rret [$(
                    $pname $pty
                )*]
            )*]
        }
        $crate::ghost_chan_new! { @inner_send_trait
            $($ameta)* ($($avis)*) $aname $aerr [$(
                ($($rmeta)*) $rname $rnamec $rret [$(
                    $pname $pty
                )*]
            )*]
        }
    };
    (   @inner_protocol
        $($ameta:meta)* ($($avis:tt)*) $aname:ident $aerr:ty [$(
            ($($rmeta:meta)*) $rname:ident $rnamec:ident $rret:ty [$(
                $pname:ident $pty:ty
            )*]
        )*]
    ) => {
        #[derive(Debug)]
        $(#[$ameta])*
        $($avis)* enum $aname {
            $(
                $(#[$rmeta])*
                $rnamec ($crate::ghost_chan::GhostChanItem<
                    ($($pty,)*),
                    ::std::result::Result<$rret, $aerr>,
                >),
            )*
        }
    };
    (   @inner_send_trait
        $($ameta:meta)* ($($avis:tt)*) $aname:ident $aerr:ty [$(
            ($($rmeta:meta)*) $rname:ident $rnamec:ident $rret:ty [$(
                $pname:ident $pty:ty
            )*]
        )*]
    ) => {
        $crate::dependencies::paste::item! {
            // -- helper functions for generating / sending / awaiting requests
            $(
                #[allow(non_snake_case, clippy::needless_lifetimes)]
                fn [< __ghost_chan_ $aname _ $rname >]<'lt, S>(
                    sender: &'lt mut S,
                    $($pname: $pty),*
                ) -> $crate::dependencies::must_future::MustBoxFuture<'lt, ::std::result::Result<$rret, $aerr>>
                where
                    S: $crate::ghost_chan::GhostChanSend<$aname> + ?Sized,
                {
                    $crate::dependencies::tracing::trace!(request = stringify!($rname));
                    let (send, recv) = $crate::dependencies::futures::channel::oneshot::channel();
                    let t = $crate::ghost_chan::GhostChanItem {
                        input: ($($pname,)*),
                        respond: Box::new(move |res| {
                            if send.send((res, $crate::dependencies::tracing::debug_span!(
                                concat!(stringify!($rname), "_respond")
                            ))).is_err() {
                                return Err($crate::GhostError::from("send error"));
                            }
                            Ok(())
                        }),
                        span: $crate::dependencies::tracing::debug_span!(stringify!($rname)),
                    };

                    let t = $aname :: $rnamec ( t );

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

            ///Import this trait to enable making async requests with associated channel type.
            $($avis)* trait [< $aname Send >]: $crate::ghost_chan::GhostChanSend<$aname> {
                $(
                    $(#[$rmeta])*
                    fn $rname(&mut self, $($pname: $pty),*) -> $crate::dependencies::must_future::MustBoxFuture<'_, ::std::result::Result<$rret, $aerr>> {
                        [< __ghost_chan_ $aname _ $rname >](self, $($pname),*)
                    }
                )*
            }

            impl<T: $crate::ghost_chan::GhostChanSend<$aname>> [< $aname Send >] for T {}
        }
    };
    (
        $(#[$ameta:meta])* pub ( $($avis:tt)* ) chan $($rest:tt)*
    ) => {
        $crate::ghost_chan_new! { @inner_tx
            #[$($ameta)*] (pub($($avis)*)) chan $($rest)*
        }
    };
    (
        $(#[$ameta:meta])* pub chan $($rest:tt)*
    ) => {
        $crate::ghost_chan_new! { @inner_tx
            #[$($ameta)*] (pub) chan $($rest)*
        }
    };
    (
        $(#[$ameta:meta])* chan $($rest:tt)*
    ) => {
        $crate::ghost_chan_new! { @inner_tx
            #[$($ameta)*] () chan $($rest)*
        }
    };
}
