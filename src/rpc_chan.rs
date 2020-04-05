/// RpcChan error type.
#[derive(Debug, thiserror::Error)]
pub enum RpcChanError {
    /// Failed to send on channel
    SendError(#[from] futures::channel::mpsc::SendError),

    /// Error sending response
    ResponseError(#[from] futures::channel::oneshot::Canceled),

    /// unspecified rpc chan error
    Other(String),
}

impl std::fmt::Display for RpcChanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<&str> for RpcChanError {
    fn from(s: &str) -> Self {
        RpcChanError::Other(s.to_string())
    }
}

impl From<RpcChanError> for () {
    fn from(_: RpcChanError) {}
}

/// Result type for RcpChan code
pub type RpcChanResult<T> = Result<T, RpcChanError>;

/// Response callback for an RpcChan message
pub type RpcChanRespond<T> = Box<dyn FnOnce(T) -> RpcChanResult<()> + 'static + Send>;

/// Container for RpcChan messages
pub struct RpcChanItem<I, O> {
    /// the request input type
    pub input: I,

    /// the response callback for responding to the request
    pub respond: RpcChanRespond<O>,

    /// a tracing span for logically following the request/response
    pub span: tracing::Span,
}

impl<I, O> std::fmt::Debug for RpcChanItem<I, O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", "RpcChanItem")
    }
}

#[macro_use]
mod rpc_chan_macros;
pub use rpc_chan_macros::*;

/// example expansion of an `rpc_chan!` macro invocation to prove out documentation.
pub mod rpc_chan_example {
    use super::*;

    rpc_chan! {
        name: pub MyEnum,
        error: RpcChanError,
        api: {
            TestMsg::test_msg("will respond with 'echo: input'", String, String),
            AddOne::add_one("will add 1 to input", u32, u32),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream::StreamExt;
    use rpc_chan_example::*;

    #[tokio::test]
    async fn test_rpc_chan_can_call_and_respond() {
        let (mut send, mut recv) = futures::channel::mpsc::channel(1);

        tokio::task::spawn(async move {
            while let Some(msg) = recv.next().await {
                match msg {
                    MyEnum::TestMsg(RpcChanItem { input, respond, .. }) => {
                        respond(Ok(format!("echo: {}", input))).unwrap();
                    }
                    MyEnum::AddOne(RpcChanItem { input, respond, .. }) => {
                        respond(Ok(input + 1)).unwrap();
                    }
                }
            }
        });

        let r = send.test_msg("hello1".to_string()).await.unwrap();
        assert_eq!("echo: hello1", &r);

        let r = send.add_one(42).await.unwrap();
        assert_eq!(43, r);
    }
}
