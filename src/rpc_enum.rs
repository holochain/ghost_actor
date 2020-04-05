/// RpcEnum error type.
#[derive(Debug, thiserror::Error)]
pub enum RpcEnumError {
    SendError(#[from] futures::channel::mpsc::SendError),
    ResponseError(#[from] futures::channel::oneshot::Canceled),
    Other(String),
}

impl std::fmt::Display for RpcEnumError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<&str> for RpcEnumError {
    fn from(s: &str) -> Self {
        RpcEnumError::Other(s.to_string())
    }
}

impl From<RpcEnumError> for () {
    fn from(_: RpcEnumError) {}
}

pub type RpcEnumResult<T> = Result<T, RpcEnumError>;

pub type RpcEnumRespond<T> = Box<dyn FnOnce(T) -> RpcEnumResult<()> + 'static + Send>;

pub struct RpcEnumType<I, O> {
    pub input: I,
    pub respond: RpcEnumRespond<O>,
    pub span: tracing::Span,
}

impl<I, O> std::fmt::Debug for RpcEnumType<I, O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", "RpcEnumType")
    }
}

#[macro_use]
mod rpc_enum_macros;
pub use rpc_enum_macros::*;

pub mod rpc_enum_example {
    use super::*;

    rpc_enum! {
        name: pub MyEnum,
        error: RpcEnumError,
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
    use rpc_enum_example::*;

    #[tokio::test]
    async fn test_rpc_enum_can_call_and_respond() {
        let (mut send, mut recv) = futures::channel::mpsc::channel(1);

        tokio::task::spawn(async move {
            while let Some(msg) = recv.next().await {
                match msg {
                    MyEnum::TestMsg(RpcEnumType { input, respond, .. }) => {
                        respond(Ok(format!("echo: {}", input))).unwrap();
                    }
                    MyEnum::AddOne(RpcEnumType { input, respond, .. }) => {
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
