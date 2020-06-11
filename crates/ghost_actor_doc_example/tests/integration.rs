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

    ghost_actor::ghost_actor! {
        pub actor MyActor<MyError> {
            fn my_fn(input: i32) -> i32;
            fn my_inner(input: i32) -> i32;
        }
    }
}

mod my_impl {
    pub struct MyImpl {
        i_s: super::my_mod::MyActorInternalSender<super::my_mod::MyChan>,
    }

    impl MyImpl {
        pub async fn spawn() -> super::my_mod::MyActorSender {
            let (sender, driver) =
                super::my_mod::MyActorSender::ghost_actor_spawn(|i_s| {
                    use ghost_actor::dependencies::futures::future::FutureExt;
                    async move { Ok(MyImpl { i_s }) }.boxed().into()
                })
                .await
                .unwrap();
            tokio::task::spawn(driver);
            sender
        }
    }

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

    impl
        super::my_mod::MyActorHandler<
            super::my_mod::MyChan,
            super::my_mod::MyChan,
        > for MyImpl
    {
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
            let mut i_s = self.i_s.clone();
            Ok(ghost_actor::dependencies::must_future::MustBoxFuture::new(
                async move {
                    use super::my_mod::MyChanSend;
                    i_s.ghost_actor_internal().my_fn(input).await
                },
            ))
        }

        fn handle_ghost_actor_internal(
            &mut self,
            input: super::my_mod::MyChan,
        ) -> super::my_mod::MyActorResult<()> {
            tokio::task::spawn(input.dispatch(self));
            Ok(())
        }

        fn handle_ghost_actor_custom(
            &mut self,
            input: super::my_mod::MyChan,
        ) -> super::my_mod::MyActorResult<()> {
            tokio::task::spawn(input.dispatch(self));
            Ok(())
        }
    }
}

#[tokio::test(threaded_scheduler)]
async fn test_ghost_actor_integration() {
    let mut sender = my_impl::MyImpl::spawn().await;

    assert_eq!(43, sender.my_fn(42).await.unwrap());
    assert_eq!(43, sender.my_inner(42).await.unwrap());

    use my_mod::MyChanSend;
    assert_eq!(
        43,
        sender
            .ghost_actor_custom::<my_mod::MyChan>()
            .my_fn(42)
            .await
            .unwrap()
    );
}
