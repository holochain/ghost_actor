#![allow(dead_code)]

#[cfg(test)]
mod tests {
    use crate::*;
    use must_future::*;

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

    ghost_event! {
        /// custom chan
        pub ghost_event MyInternalChan<MyError> {
            /// will respond with 'echo: input'.
            fn test_msg(input: String) -> String;
        }
    }

    ghost_event! {
        /// this is my custom actor doc
        pub ghost_event MyActor<MyError> {
            /// A test message, sends a String, receives a String.
            fn test_message(input: String) -> String;

            /// A test function, output adds 1 to input.
            fn add_one(input: u32) -> u32;

            /// Ensure we can take params that don't implement Debug.
            fn req_not_debug(input: NotDebug) -> ();

            /// Makes an internal_sender request from outside. In reality, you'd never need a command like this.
            fn funky_internal(input: String) -> String;

            /// Calls internal ghost_actor_shutdown_immediate() command. In reality, you'd never need a command like this.
            fn funky_stop() -> ();
        }
    }

    /// An example implementation of the example MyActor GhostActor.
    struct MyActorImpl {
        internal_sender: GhostSender<MyInternalChan>,
        did_shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
    }

    /// All handlers must implement this trait.
    /// (provides the handle_ghost_actor_shutdown callback)
    impl GhostControlHandler for MyActorImpl {
        fn handle_ghost_actor_shutdown(&mut self) {
            self.did_shutdown
                .store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    impl GhostHandler<MyInternalChan> for MyActorImpl {}

    impl MyInternalChanHandler for MyActorImpl {
        fn handle_test_msg(
            &mut self,
            input: String,
        ) -> MyInternalChanHandlerResult<String> {
            Ok(async move { Ok(format!("internal respond to: {}", input)) }
                .must_box())
        }
    }

    impl GhostHandler<MyActor> for MyActorImpl {}

    impl MyActorHandler for MyActorImpl {
        fn handle_test_message(
            &mut self,
            input: String,
        ) -> MyActorHandlerResult<String> {
            Ok(async move { Ok(format!("echo: {}", input)) }.must_box())
        }

        fn handle_add_one(&mut self, input: u32) -> MyActorHandlerResult<u32> {
            Ok(async move { Ok(input + 1) }.must_box())
        }

        fn handle_req_not_debug(
            &mut self,
            _input: NotDebug,
        ) -> MyActorHandlerResult<()> {
            Ok(async move { Ok(()) }.must_box())
        }

        fn handle_funky_internal(
            &mut self,
            input: String,
        ) -> MyActorHandlerResult<String> {
            let fut = self.internal_sender.test_msg(input);
            Ok(async move { Ok(fut.await.unwrap()) }.must_box())
        }

        fn handle_funky_stop(&mut self) -> MyActorHandlerResult<()> {
            let fut = self.internal_sender.ghost_actor_shutdown_immediate();
            Ok(async move { Ok(fut.await.unwrap()) }.must_box())
        }
    }

    impl MyActorImpl {
        /// Rather than using ghost_actor_spawn directly, use this simple spawn.
        pub async fn spawn() -> Result<
            (
                GhostSender<MyActor>,
                std::sync::Arc<std::sync::atomic::AtomicBool>,
            ),
            MyError,
        > {
            let did_shutdown =
                std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let did_shutdown_clone = did_shutdown.clone();

            let builder = actor_builder::GhostActorBuilder::new();

            let sender = builder
                .channel_factory()
                .create_channel::<MyActor>()
                .await
                .unwrap();

            let internal_sender = builder
                .channel_factory()
                .create_channel::<MyInternalChan>()
                .await
                .unwrap();

            tokio::task::spawn(builder.spawn(MyActorImpl {
                internal_sender,
                did_shutdown,
            }));

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
    async fn it_can_use_eq_on_senders() {
        let (sender_a1, _) = MyActorImpl::spawn().await.unwrap();
        let (sender_b1, _) = MyActorImpl::spawn().await.unwrap();
        let sender_a2 = sender_a1.clone();
        assert!(sender_a1 == sender_a2);
        assert!(sender_a1 != sender_b1);
    }

    #[tokio::test]
    async fn it_can_hash_senders() {
        let (sender_a1, _) = MyActorImpl::spawn().await.unwrap();
        let (sender_b1, _) = MyActorImpl::spawn().await.unwrap();
        let sender_a2 = sender_a1.clone();
        let mut set = std::collections::HashSet::new();
        assert!(set.insert(sender_a1));
        assert!(set.insert(sender_b1));
        assert!(!set.insert(sender_a2));
        assert_eq!(2, set.len());
    }

    #[tokio::test]
    async fn it_check_echo() {
        init_tracing();

        let (sender, _) = MyActorImpl::spawn().await.unwrap();

        assert_eq!(
            "echo: test",
            &sender.test_message("test".to_string()).await.unwrap(),
        );
    }

    #[tokio::test]
    async fn it_check_add_1() {
        init_tracing();

        let (sender, _) = MyActorImpl::spawn().await.unwrap();

        assert_eq!(43, sender.add_one(42).await.unwrap());
    }

    #[tokio::test]
    async fn it_check_internal() {
        init_tracing();

        let (sender, _) = MyActorImpl::spawn().await.unwrap();

        assert_eq!(
            "internal respond to: i_test",
            &sender.funky_internal("i_test".into()).await.unwrap(),
        );
    }

    #[tokio::test]
    async fn it_check_shutdown() {
        init_tracing();

        let (sender, did_shutdown) = MyActorImpl::spawn().await.unwrap();

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

        let (sender, did_shutdown) = MyActorImpl::spawn().await.unwrap();

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
