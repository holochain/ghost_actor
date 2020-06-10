#[cfg(test)]
mod tests {
    use crate::*;
    use futures::future::FutureExt;

    /// Custom example error type.
    #[derive(Debug, thiserror::Error)]
    pub enum MyError {
        /// custom errors must support `From<GhostError>`
        GhostError(#[from] GhostError),
    }

    /// This struct does not implement debug.
    pub struct NotDebug;

    impl std::fmt::Display for MyError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    ghost_chan! {
        /// custom chan
        pub chan MyCustomChan<MyError> {
            /// will respond with 'echo: input'.
            fn test_msg(input: String) -> String;
        }
    }

    ghost_chan! {
        /// custom chan
        pub chan MyInternalChan<MyError> {
            /// will respond with 'echo: input'.
            fn test_msg(input: String) -> String;
        }
    }

    ghost_actor! {
        /// this is my custom actor doc
        pub actor MyActor<MyError> {
            /// A test message, sends a String, receives a String.
            fn test_message(input: String) -> String;

            /// A test function, output adds 1 to input.
            fn add_one(input: u32) -> u32;

            /// Ensure we can take params that don't implement Debug.
            #[allow(dead_code)]
            fn req_not_debug(input: NotDebug) -> ();

            /// Makes an internal_sender request from outside. In reality, you'd never need a command like this.
            fn funky_internal(input: String) -> String;

            /// Calls internal ghost_actor_shutdown_immediate() command. In reality, you'd never need a command like this.
            fn funky_stop() -> ();
        }
    }

    /// An example implementation of the example MyActor GhostActor.
    struct MyActorImpl {
        internal_sender: MyActorInternalSender<MyInternalChan>,
        did_shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
    }

    impl MyCustomChanHandler for MyActorImpl {
        fn handle_test_msg(
            &mut self,
            input: String,
        ) -> MyCustomChanHandlerResult<String> {
            Ok(async move { Ok(format!("custom respond to: {}", input)) }
                .boxed()
                .into())
        }
    }

    impl MyInternalChanHandler for MyActorImpl {
        fn handle_test_msg(
            &mut self,
            input: String,
        ) -> MyInternalChanHandlerResult<String> {
            Ok(async move { Ok(format!("internal respond to: {}", input)) }
                .boxed()
                .into())
        }
    }

    impl MyActorHandler<MyCustomChan, MyInternalChan> for MyActorImpl {
        fn handle_test_message(
            &mut self,
            input: String,
        ) -> MyActorHandlerResult<String> {
            Ok(async move { Ok(format!("echo: {}", input)) }.boxed().into())
        }

        fn handle_add_one(&mut self, input: u32) -> MyActorHandlerResult<u32> {
            Ok(async move { Ok(input + 1) }.boxed().into())
        }

        fn handle_req_not_debug(
            &mut self,
            _input: NotDebug,
        ) -> MyActorHandlerResult<()> {
            Ok(async move { Ok(()) }.boxed().into())
        }

        fn handle_funky_internal(
            &mut self,
            input: String,
        ) -> MyActorHandlerResult<String> {
            let mut i_s = self.internal_sender.clone();
            Ok(async move {
                Ok(i_s.ghost_actor_internal().test_msg(input).await.unwrap())
            }
            .boxed()
            .into())
        }

        fn handle_funky_stop(&mut self) -> MyActorHandlerResult<()> {
            self.internal_sender.ghost_actor_shutdown_immediate();
            Ok(async move { Ok(()) }.boxed().into())
        }

        fn handle_ghost_actor_shutdown(&mut self) {
            self.did_shutdown
                .store(true, std::sync::atomic::Ordering::SeqCst);
        }

        fn handle_ghost_actor_custom(
            &mut self,
            input: MyCustomChan,
        ) -> MyActorResult<()> {
            tokio::task::spawn(input.dispatch(self));
            Ok(())
        }

        fn handle_ghost_actor_internal(
            &mut self,
            input: MyInternalChan,
        ) -> MyActorResult<()> {
            tokio::task::spawn(input.dispatch(self));
            Ok(())
        }
    }

    impl MyActorImpl {
        /// Rather than using ghost_actor_spawn directly, use this simple spawn.
        pub async fn spawn() -> Result<
            (MyActorSender, std::sync::Arc<std::sync::atomic::AtomicBool>),
            MyError,
        > {
            let did_shutdown =
                std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let did_shutdown_clone = did_shutdown.clone();
            let (sender, driver) =
                MyActorSender::ghost_actor_spawn(Box::new(|i_s| {
                    async move {
                        Ok(MyActorImpl {
                            internal_sender: i_s,
                            did_shutdown,
                        })
                    }
                    .boxed()
                    .into()
                }))
                .await?;
            tokio::task::spawn(driver);
            Ok((sender, did_shutdown_clone))
        }
    }

    fn init_tracing() {
        let _ = tracing::subscriber::set_global_default(
            tracing_subscriber::FmtSubscriber::builder()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::from_default_env(),
                )
                .compact()
                .finish(),
        );
    }

    #[tokio::test]
    async fn it_check_echo() {
        init_tracing();

        let (mut sender, _) = MyActorImpl::spawn().await.unwrap();

        assert_eq!(
            "echo: test",
            &sender.test_message("test".to_string()).await.unwrap(),
        );
    }

    #[tokio::test]
    async fn it_check_add_1() {
        init_tracing();

        let (mut sender, _) = MyActorImpl::spawn().await.unwrap();

        assert_eq!(43, sender.add_one(42).await.unwrap());
    }

    #[tokio::test]
    async fn it_check_custom() {
        init_tracing();

        let (mut sender, _) = MyActorImpl::spawn().await.unwrap();

        assert_eq!(
            "custom respond to: c_test",
            &sender
                .ghost_actor_custom::<MyCustomChan>()
                .test_msg("c_test".into())
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn it_check_internal() {
        init_tracing();

        let (mut sender, _) = MyActorImpl::spawn().await.unwrap();

        assert_eq!(
            "internal respond to: i_test",
            &sender.funky_internal("i_test".into()).await.unwrap(),
        );
    }

    #[tokio::test]
    async fn it_check_shutdown() {
        init_tracing();

        let (mut sender, did_shutdown) = MyActorImpl::spawn().await.unwrap();

        sender.ghost_actor_shutdown().await.unwrap();

        let res = format!("{:?}", sender.add_one(42).await);
        if &res
            != "Err(GhostError(SendError(SendError { kind: Disconnected })))"
            && &res != "Err(GhostError(ResponseError(Canceled)))"
        {
            panic!("expected send error");
        }

        assert!(did_shutdown.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn it_check_internal_shutdown() {
        init_tracing();

        let (mut sender, did_shutdown) = MyActorImpl::spawn().await.unwrap();

        sender.funky_stop().await.unwrap();

        let res = format!("{:?}", sender.add_one(42).await);
        if &res
            != "Err(GhostError(SendError(SendError { kind: Disconnected })))"
            && &res != "Err(GhostError(ResponseError(Canceled)))"
        {
            panic!("expected send error");
        }

        assert!(did_shutdown.load(std::sync::atomic::Ordering::SeqCst));
    }
}
