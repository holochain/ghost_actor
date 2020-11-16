/// The `ghost_chan!` macro generates an enum and helper types that make it
/// easy to make inline async requests and await responses.
#[macro_export]
macro_rules! ghost_chan {
    // using @inner_ self references so we don't have to export / pollute
    // a bunch of sub macros.

    // -- inner_tx does some translation from our external macro api
    // -- to a simpler internal api

    (   @inner_tx
        $(#[$ameta:meta])*
        ($($avis:tt)*) chan $aname:ident<$aerr:ty> {
            $(
                $(#[$rmeta:meta])* fn $rname:ident ( $($pname:ident: $pty:ty),* $(,)? ) -> $rret:ty;
            )*
        }
    ) => {
        $crate::dependencies::paste::item! {
            $crate::ghost_chan! { @inner
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
            /// Result Type
            $($avis)* type [< $aname Result >] <T> = ::std::result::Result<T, $aerr>;

            /// Future Type.
            $($avis)* type [< $aname Future >] <T> = $crate::dependencies::must_future::MustBoxFuture<'static, [< $aname Result >] <T> >;

            /// Handler Result Type.
            $($avis)* type [< $aname HandlerResult >] <T> = ::std::result::Result<[< $aname Future >] <T>, $aerr>;
        }

        $crate::ghost_chan! { @inner_protocol
            ($($ameta)*) ($($avis)*) $aname $aerr [$(
                ($($rmeta)*) $rname $rnamec $rret [$(
                    $pname $pty
                )*]
            )*]
        }
        $crate::ghost_chan! { @inner_send_trait
            ($($ameta)*) ($($avis)*) $aname $aerr [$(
                ($($rmeta)*) $rname $rnamec $rret [$(
                    $pname $pty
                )*]
            )*]
        }
        $crate::ghost_chan! { @inner_handler_trait
            ($($ameta)*) ($($avis)*) $aname $aerr [$(
                ($($rmeta)*) $rname $rnamec $rret [$(
                    $pname $pty
                )*]
            )*]
        }
    };

    // -- write the enum item -- //

    (   @inner_protocol
        ($($ameta:meta)*) ($($avis:tt)*) $aname:ident $aerr:ty [$(
            ($($rmeta:meta)*) $rname:ident $rnamec:ident $rret:ty [$(
                $pname:ident $pty:ty
            )*]
        )*]
    ) => {
        $crate::dependencies::paste::item! {
            // -- the main enum item -- //

            $(#[$ameta])*
            $($avis)* enum $aname {
                $(
                    $(#[$rmeta])*
                    $rnamec {
                        /// Tracing span from request invocation.
                        span_context: $crate::dependencies::observability::Context,

                        /// Response callback - respond to the request.
                        respond: $crate::GhostRespond<
                            [< $aname HandlerResult >] <$rret>,
                        >,

                        $(
                            /// Input parameter.
                            $pname: $pty,
                        )*
                    },
                )*
            }

            impl $crate::GhostEvent for $aname {}

            impl<H: [< $aname Handler >]> $crate::GhostDispatch<H> for $aname {
                fn ghost_actor_dispatch(self, h: &mut H) {
                    match self {
                        $(
                            $aname::$rnamec { span_context, respond, $($pname,)* } => {
                                let span = $crate::dependencies::tracing::trace_span!(concat!("handle_", stringify!($rname)));
                                let _g = span.enter();
                                $crate::dependencies::observability::OpenSpanExt::set_context(&span, span_context);
                                respond.respond(h.[< handle_ $rname >]($($pname,)*));
                            }
                        )*
                    }
                }
            }

            // -- implement debug - note this does not expose the parameters
            // -- because we don't want to require them to be Debug

            impl ::std::fmt::Debug for $aname {
                fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    match self {
                        $(
                            $aname :: $rnamec { .. } => {
                                write!(
                                    f,
                                    "{}::{} {{ .. }}",
                                    stringify!($aname),
                                    stringify!($rnamec),
                                )
                            }
                        )*
                    }
                }
            }
        }
    };

    // -- write the "Sender" trait that exposes user-friendly,
    // -- ergonomic async request functions

    (   @inner_send_trait
        ($($ameta:meta)*) ($($avis:tt)*) $aname:ident $aerr:ty [$(
            ($($rmeta:meta)*) $rname:ident $rnamec:ident $rret:ty [$(
                $pname:ident $pty:ty
            )*]
        )*]
    ) => {
        $crate::dependencies::paste::item! {
            $(#[$ameta])*
            $($avis)* trait [< $aname Sender >]: $crate::GhostChannelSender<$aname> {
                $(
                    $(#[$rmeta])*
                    fn $rname(&self, $($pname: $pty),*) -> [< $aname Future >] <$rret> {
                        use $crate::dependencies::observability::OpenSpanExt;
                        let (s, r) = $crate::dependencies::futures::channel::oneshot::channel();
                        let span_context = $crate::dependencies::tracing::Span::get_current_context();
                        let t = $aname::$rnamec {
                            span_context,
                            respond: $crate::GhostRespond::new(
                                s,
                                concat!(stringify!($rname), "_respond"),
                            ),
                            $($pname: $pname,)*
                        };
                        let send_fut = self.ghost_actor_channel_send(t);
                        $crate::dependencies::must_future::MustBoxFuture::new(async move {
                            send_fut.await?;
                            let (r, span_context) = r.await.map_err($crate::GhostError::from)?;
                            $crate::dependencies::tracing::Span::set_current_context(span_context);
                            r?.await
                        })
                    }
                )*
            }

            impl<S: $crate::GhostChannelSender<$aname>> [< $aname Sender >] for S {}
        }
    };

    // -- write the "ChanHandler" trait

    (   @inner_handler_trait
        ($($ameta:meta)*) ($($avis:tt)*) $aname:ident $aerr:ty [$(
            ($($rmeta:meta)*) $rname:ident $rnamec:ident $rret:ty [$(
                $pname:ident $pty:ty
            )*]
        )*]
    ) => {
        $crate::dependencies::paste::item! {
            $(#[$ameta])*
            $($avis)* trait [< $aname Handler >]: $crate::GhostHandler<$aname> {
                $(
                    $(#[$rmeta])*
                    fn [< handle_ $rname >] (
                        &mut self, $($pname: $pty,)*
                    ) -> [< $aname HandlerResult >]<$rret>;
                )*
            }
        }
    };

    // -- visibility helpers - these are the arms users actually invoke -- //

    // specialized pub visibility
    (
        $(#[$ameta:meta])* pub ( $($avis:tt)* ) chan $($rest:tt)*
    ) => {
        $crate::ghost_chan! { @inner_tx
            $(#[$ameta])* (pub($($avis)*)) chan $($rest)*
        }
    };

    // generic pub visibility
    (
        $(#[$ameta:meta])* pub chan $($rest:tt)*
    ) => {
        $crate::ghost_chan! { @inner_tx
            $(#[$ameta])* (pub) chan $($rest)*
        }
    };

    // private visibility
    (
        $(#[$ameta:meta])* chan $($rest:tt)*
    ) => {
        $crate::ghost_chan! { @inner_tx
            $(#[$ameta])* () chan $($rest)*
        }
    };
}
