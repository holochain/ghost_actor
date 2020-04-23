#[allow(clippy::needless_doctest_main)]
/// Example usage for unit testing and showing documentation generation.
///
/// ```
/// use ghost_actor::example::MyError;
/// ghost_actor::ghost_chan! {
///     name: pub MyCustomChan,
///     error: MyError,
///     api: {
///         TestMsg::test_msg("will respond with 'echo: input'", String, String),
///     }
/// }
///
/// ghost_actor::ghost_chan! {
///     name: pub MyInternalChan,
///     error: MyError,
///     api: {
///         TestMsg::test_msg("will respond with 'echo: input'", String, String),
///     }
/// }
///
/// ghost_actor::ghost_actor! {
///     name: pub MyActor,
///     error: MyError,
///     api: {
///         TestMessage::test_message(
///             "A test message, sends a String, receives a String.",
///             String, String),
///         AddOne::add_one(
///             "A test function, output adds 1 to input.",
///             u32, u32),
///         FunkyInternal::funky_internal(
///             "Makes an internal_sender request from outside. In reality, you'd never need a command like this.",
///             String, must_future::MustBoxFuture<'static, String>),
///         FunkyStop::funky_stop(
///             "Calls internal ghost_actor_shutdown_immediate() command. In reality, you'd never need a command like this.",
///             (), ()),
///     }
/// }
/// # pub fn main() {}
/// ```
pub mod example {
    /// Custom example error type.
    #[derive(Debug, thiserror::Error)]
    pub enum MyError {
        /// custom errors must support `From<GhostError>`
        GhostError(#[from] crate::GhostError),
    }

    impl std::fmt::Display for MyError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    crate::ghost_chan! {
        name: pub MyCustomChan,
        error: MyError,
        api: {
            TestMsg::test_msg("will respond with 'echo: input'", String, String),
        }
    }

    crate::ghost_chan! {
        name: pub MyInternalChan,
        error: MyError,
        api: {
            TestMsg::test_msg("will respond with 'echo: input'", String, String),
        }
    }

    crate::ghost_actor! {
        name: pub MyActor,
        error: MyError,
        api: {
            TestMessage::test_message(
                "A test message, sends a String, receives a String.",
                String, String),
            AddOne::add_one(
                "A test function, output adds 1 to input.",
                u32, u32),
            FunkyInternal::funky_internal(
                "Makes an internal_sender request from outside. In reality, you'd never need a command like this.",
                String, must_future::MustBoxFuture<'static, String>),
            FunkyStop::funky_stop(
                "Calls internal ghost_actor_shutdown_immediate() command. In reality, you'd never need a command like this.",
                (), ()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use example::*;

    /// An example implementation of the example MyActor GhostActor.
    struct MyActorImpl {
        internal_sender: MyActorInternalSender<MyCustomChan, MyInternalChan>,
    }

    impl MyActorHandler<MyCustomChan, MyInternalChan> for MyActorImpl {
        fn handle_test_message(&mut self, input: String) -> Result<String, MyError> {
            Ok(format!("echo: {}", input))
        }

        fn handle_add_one(&mut self, input: u32) -> Result<u32, MyError> {
            Ok(input + 1)
        }

        fn handle_funky_internal(
            &mut self,
            input: String,
        ) -> Result<must_future::MustBoxFuture<'static, String>, MyError> {
            use futures::future::FutureExt;

            let mut i_s = self.internal_sender.clone();
            Ok(
                async move { i_s.ghost_actor_internal().test_msg(input).await.unwrap() }
                    .boxed()
                    .into(),
            )
        }

        fn handle_funky_stop(&mut self) -> Result<(), MyError> {
            self.internal_sender.ghost_actor_shutdown_immediate();
            Ok(())
        }

        fn handle_ghost_actor_custom(&mut self, input: MyCustomChan) {
            match input {
                MyCustomChan::TestMsg(GhostChanItem { input, respond, .. }) => {
                    respond(Ok(format!("custom respond to: {}", input))).unwrap();
                }
            }
        }

        fn handle_ghost_actor_internal(&mut self, input: MyInternalChan) {
            match input {
                MyInternalChan::TestMsg(GhostChanItem { input, respond, .. }) => {
                    respond(Ok(format!("internal respond to: {}", input))).unwrap();
                }
            }
        }
    }

    impl MyActorImpl {
        /// Rather than using ghost_actor_spawn directly, use this simple spawn.
        pub async fn spawn() -> Result<MyActorSender<MyCustomChan>, MyError> {
            use futures::future::FutureExt;
            let (sender, driver) = MyActorSender::ghost_actor_spawn(Box::new(|i_s| {
                async move {
                    Ok(MyActorImpl {
                        internal_sender: i_s,
                    })
                }
                .boxed()
                .into()
            }))
            .await?;
            tokio::task::spawn(driver);
            Ok(sender)
        }
    }

    fn init_tracing() {
        let _ = tracing::subscriber::set_global_default(
            tracing_subscriber::FmtSubscriber::builder()
                .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                .compact()
                .finish(),
        );
    }

    #[tokio::test]
    async fn it_check_echo() {
        init_tracing();

        let mut sender = MyActorImpl::spawn().await.unwrap();

        assert_eq!(
            "echo: test",
            &sender.test_message("test".to_string()).await.unwrap(),
        );
    }

    #[tokio::test]
    async fn it_check_add_1() {
        init_tracing();

        let mut sender = MyActorImpl::spawn().await.unwrap();

        assert_eq!(43, sender.add_one(42).await.unwrap());
    }

    #[tokio::test]
    async fn it_check_custom() {
        init_tracing();

        let mut sender = MyActorImpl::spawn().await.unwrap();

        assert_eq!(
            "custom respond to: c_test",
            &sender
                .ghost_actor_custom()
                .test_msg("c_test".into())
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn it_check_internal() {
        init_tracing();

        let mut sender = MyActorImpl::spawn().await.unwrap();

        assert_eq!(
            "internal respond to: i_test",
            &sender.funky_internal("i_test".into()).await.unwrap().await,
        );
    }

    #[tokio::test]
    async fn it_check_shutdown() {
        init_tracing();

        let mut sender = MyActorImpl::spawn().await.unwrap();

        sender.ghost_actor_shutdown().await.unwrap();

        assert_eq!(
            "Err(GhostError(SendError(SendError { kind: Disconnected })))",
            &format!("{:?}", sender.add_one(42).await),
        );
    }

    #[tokio::test]
    async fn it_check_internal_shutdown() {
        init_tracing();

        let mut sender = MyActorImpl::spawn().await.unwrap();

        sender.funky_stop().await.unwrap();

        assert_eq!(
            "Err(GhostError(SendError(SendError { kind: Disconnected })))",
            &format!("{:?}", sender.add_one(42).await),
        );
    }
}
