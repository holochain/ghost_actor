#[allow(clippy::needless_doctest_main)]
/// Example usage for unit testing and showing documentation generation.
///
/// ```
/// use ghost_actor::example::MyError;
/// ghost_actor::ghost_actor! {
///     name: pub MyActor,
///     error: MyError,
///     api: {
///         test(
///             "A test message, sends a String, receives a String.",
///             String, String),
///         add1(
///             "A test function, output adds 1 to input.",
///             u32, u32),
///         stop(
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
        GhostActorError(#[from] crate::GhostActorError),
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
            test(
                "A test message, sends a String, receives a String.",
                String, String),
            add1(
                "A test function, output adds 1 to input.",
                u32, u32),
            stop(
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
        fn handle_test(
            &mut self,
            _: &mut MyActorInternalSender<(), ()>,
            input: String,
        ) -> Result<String, MyError> {
            Ok(format!("echo: {}", input))
        }

        fn handle_add1(
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

    #[tokio::test]
    async fn it_works() {
        let mut sender = MyActorImpl::spawn();

        assert_eq!(
            "echo: test",
            &sender.test("test".to_string()).await.unwrap(),
        );

        assert_eq!(43, sender.add1(42).await.unwrap(),);

        sender.stop(()).await.unwrap();

        assert_eq!(
            "Err(GhostActorError(SendError(SendError { kind: Disconnected })))",
            &format!("{:?}", sender.add1(42).await),
        );
    }
}
