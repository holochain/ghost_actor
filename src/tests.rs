#[allow(clippy::needless_doctest_main)]
/// Example usage for unit testing and showing documentation generation.
///
/// ```
/// use ghost_actor::example::MyError;
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
///         Stop::stop(
///             "Calls internal shutdown() command.",
///             (), ()),
///     }
/// }
/// # pub fn main() {}
/// ```
pub mod example {
    /// Custom example error type.
    #[derive(Debug, thiserror::Error)]
    pub enum MyError {
        /// custom errors must support `From<GhostActorError>`
        GhostActorError(#[from] crate::GhostActorError),

        /// TODO - let's just have one error type
        /// custom errors must support `From<RpcChanError>`
        RpcChanError(#[from] crate::RpcChanError),
    }

    impl std::fmt::Display for MyError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self)
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
            Stop::stop(
                "Calls internal shutdown() command.",
                (), ()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use example::*;

    /// An example implementation of the example MyActor GhostActor.
    struct MyActorImpl;

    impl MyActorHandler<(), ()> for MyActorImpl {
        fn handle_test_message(
            &mut self,
            _: &mut MyActorInternalSender<(), ()>,
            input: String,
        ) -> Result<String, MyError> {
            Ok(format!("echo: {}", input))
        }

        fn handle_add_one(
            &mut self,
            _: &mut MyActorInternalSender<(), ()>,
            input: u32,
        ) -> Result<u32, MyError> {
            Ok(input + 1)
        }

        fn handle_stop(
            &mut self,
            internal: &mut MyActorInternalSender<(), ()>,
            _: (),
        ) -> Result<(), MyError> {
            internal.shutdown();
            Ok(())
        }
    }

    impl MyActorImpl {
        /// Rather than using ghost_actor_spawn directly, use this simple spawn.
        pub fn spawn() -> MyActorSender<()> {
            let (sender, driver) = MyActorSender::ghost_actor_spawn(MyActorImpl);
            tokio::task::spawn(driver);
            sender
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

        let mut sender = MyActorImpl::spawn();

        assert_eq!(
            "echo: test",
            &sender.test_message("test".to_string()).await.unwrap(),
        );
    }

    #[tokio::test]
    async fn it_check_add_1() {
        init_tracing();

        let mut sender = MyActorImpl::spawn();

        assert_eq!(43, sender.add_one(42).await.unwrap(),);
    }

    #[tokio::test]
    async fn it_check_shutdown() {
        init_tracing();

        let mut sender = MyActorImpl::spawn();

        sender.ghost_actor_shutdown().await.unwrap();

        assert_eq!(
            "Err(GhostActorError(SendError(SendError { kind: Disconnected })))",
            &format!("{:?}", sender.add_one(42).await),
        );
    }

    #[tokio::test]
    async fn it_check_custom_stop() {
        init_tracing();

        let mut sender = MyActorImpl::spawn();

        sender.stop(()).await.unwrap();

        assert_eq!(
            "Err(GhostActorError(SendError(SendError { kind: Disconnected })))",
            &format!("{:?}", sender.add_one(42).await),
        );
    }
}
