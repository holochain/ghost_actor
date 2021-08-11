// This integration suite is mainly to ensure the macros function without
// any assumed `use`s.

mod my_mod {
    #[derive(Debug, thiserror::Error)]
    pub enum MyError {
        #[error("GhostError: {0}")]
        GhostError(#[from] ghost_actor::GhostError),
    }

    ghost_actor::ghost_chan! {
        pub chan MyChan<MyError> {
            fn my_fn(input: i32) -> i32;
        }
    }

    ghost_actor::ghost_chan! {
        pub chan MyActor<MyError> {
            fn my_fn(input: i32) -> i32;
            fn my_inner(input: i32) -> i32;
        }
    }
}

mod my_impl {
    pub struct MyImpl {
        i_s: ghost_actor::GhostSender<super::my_mod::MyChan>,
    }

    impl MyImpl {
        pub async fn spawn() -> ghost_actor::GhostSender<super::my_mod::MyActor>
        {
            let builder = ghost_actor::actor_builder::GhostActorBuilder::new();
            let sender = builder
                .channel_factory()
                .create_channel::<super::my_mod::MyActor>()
                .await
                .unwrap();
            let i_s = builder
                .channel_factory()
                .create_channel::<super::my_mod::MyChan>()
                .await
                .unwrap();
            tokio::task::spawn(builder.spawn(MyImpl { i_s }));
            sender
        }
    }

    impl ghost_actor::GhostControlHandler for MyImpl {}

    impl ghost_actor::GhostHandler<super::my_mod::MyChan> for MyImpl {}

    impl super::my_mod::MyChanHandler for MyImpl {
        fn handle_my_fn(
            &mut self,
            input: i32,
        ) -> super::my_mod::MyChanHandlerResult<i32> {
            Ok(ghost_actor::dependencies::must_future::MustBoxFuture::new(
                async move { Ok(input + 1) },
            ))
        }
    }

    impl ghost_actor::GhostHandler<super::my_mod::MyActor> for MyImpl {}

    impl super::my_mod::MyActorHandler for MyImpl {
        fn handle_my_fn(
            &mut self,
            input: i32,
        ) -> super::my_mod::MyActorHandlerResult<i32> {
            Ok(ghost_actor::dependencies::must_future::MustBoxFuture::new(
                async move { Ok(input + 1) },
            ))
        }

        fn handle_my_inner(
            &mut self,
            input: i32,
        ) -> super::my_mod::MyActorHandlerResult<i32> {
            let i_s = self.i_s.clone();
            Ok(ghost_actor::dependencies::must_future::MustBoxFuture::new(
                async move {
                    use super::my_mod::MyChanSender;
                    i_s.my_fn(input).await
                },
            ))
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_ghost_actor_integration() {
    let sender = my_impl::MyImpl::spawn().await;

    use my_mod::MyActorSender;

    assert_eq!(43, sender.my_fn(42).await.unwrap());
    assert_eq!(43, sender.my_inner(42).await.unwrap());
}
