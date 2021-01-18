use crate::*;
use tracing::Instrument;

#[tokio::test]
async fn concrete_invoke_async() {
    observability::test_run().ok();

    let (actor, driver) = GhostActor::new(42_u8);
    tokio::task::spawn(driver);

    assert_eq!(
        42,
        actor
            .invoke_async(|i| {
                let out = *i;
                Ok(resp(async move { <Result<u8, GhostError>>::Ok(out) }))
            })
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn box_invoke_async() {
    observability::test_run().ok();

    let (actor, driver) = GhostActor::new(42_u8);
    tokio::task::spawn(driver);
    let actor = actor.to_boxed();

    assert_eq!(
        42,
        actor
            .invoke_async(|i| {
                let out = *i;
                Ok(resp(async move { <Result<u8, GhostError>>::Ok(out) }))
            })
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn debuggable() {
    observability::test_run().ok();

    struct Bob;
    let b = Box::new(Bob);
    let (a, _) = GhostActor::new(b);
    let dbg = format!("{:?}", a);
    assert!(dbg.contains("GhostActor {"));
    assert!(dbg.contains("type:"));
    assert!(dbg.contains("hash:"));
    assert!(dbg.contains("Bob"));
    assert!(dbg.contains("Box"));
}

#[tokio::test]
async fn caller_send_drop_no_panic() {
    observability::test_run().ok();

    let (msg, driver) = GhostActor::new("".to_string());
    tokio::task::spawn(driver);

    let fut = {
        // set up a parent tracing context that will be dropped
        let _g = tracing::warn_span!("box_ghost_actor_test");
        let _g = _g.enter();

        let mut fut = msg.invoke(|msg: &mut String| {
            msg.push_str("Hello ");
            tracing::warn!(?msg);
            <Result<String, GhostError>>::Ok(msg.clone())
        });

        // poll once, then drop
        futures::future::poll_fn(move |cx| {
            let fut = &mut fut;
            futures::pin_mut!(fut);
            match std::future::Future::poll(fut, cx) {
                std::task::Poll::Pending => (),
                _ => panic!(),
            };
            std::task::Poll::Ready(())
        })
        .await;

        msg.invoke(|msg: &mut String| {
            msg.push_str("World!");
            tracing::warn!(?msg);
            <Result<String, GhostError>>::Ok(msg.clone())
        })
    };

    assert_eq!("Hello World!", &fut.await.unwrap());
}

#[tokio::test]
async fn box_ghost_actor_test() {
    observability::test_run().ok();

    let (msg, driver) = GhostActor::new("".to_string());
    tokio::task::spawn(driver);
    let msg: BoxGhostActor = msg.to_boxed();

    let _g = tracing::warn_span!("box_ghost_actor_test");
    let _g = _g.enter();

    msg.invoke(|msg: &mut String| {
        msg.push_str("Hello ");
        tracing::warn!(?msg);
        <Result<(), GhostError>>::Ok(())
    })
    .await
    .unwrap();

    assert_eq!(
        "Hello World!",
        &msg.invoke(|msg: &mut String| {
            msg.push_str("World!");
            tracing::warn!(?msg);
            <Result<String, GhostError>>::Ok(msg.clone())
        })
        .await
        .unwrap()
    );
}

#[tokio::test]
async fn full_actor_workflow_test() {
    observability::test_run().ok();

    trait Fruit {
        fn eat(&self) -> GhostFuture<String, GhostError>;
    }

    pub struct Banana(GhostActor<u32>);

    impl Banana {
        pub fn new() -> Self {
            let (actor, driver) = GhostActor::new(0);
            tokio::task::spawn(driver);
            Self(actor)
        }
    }

    impl Fruit for Banana {
        fn eat(&self) -> GhostFuture<String, GhostError> {
            let actor = self.0.clone();

            resp(
                async move {
                    tracing::warn!("Driving Banana.eat() future");

                    let count = actor
                        .invoke::<_, GhostError, _>(|count| {
                            *count += 1;
                            tracing::warn!(?count, "banana.increment_count");
                            Ok(*count)
                        })
                        .await?;

                    Ok(format!("ate {} bananas", count))
                }
                .instrument(tracing::warn_span!("banana.eat")),
            )
        }
    }

    let banana = Banana::new();
    assert_eq!("ate 1 bananas", &banana.eat().await.unwrap());

    let fruit: Box<dyn Fruit> = Box::new(banana);
    assert_eq!("ate 2 bananas", &fruit.eat().await.unwrap());
}
