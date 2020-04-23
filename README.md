![Crates.io](https://img.shields.io/crates/l/ghost_actor)
![Crates.io](https://img.shields.io/crates/v/ghost_actor)

# ghost_actor

A simple, ergonomic, idiomatic, macro for generating the boilerplate to use rust futures tasks in a concurrent actor style.

## Example

```rust
ghost_actor::ghost_actor! {
    name: pub MyActor,
    error: MyError,
    api: {
        AddOne::add_one(
            "A test function, output adds 1 to input.",
            u32, u32),
    }
}

/// An example implementation of the example MyActor GhostActor.
struct MyActorImpl;

impl MyActorHandler<(), ()> for MyActorImpl {
    fn handle_add_one(
        &mut self,
        input: u32,
    ) -> MyActorHandlerResult<u32> {
        Ok(async move {
            Ok(input + 1)
        }.boxed().into())
    }
}

impl MyActorImpl {
    /// Rather than using ghost_actor_spawn directly, use this simple spawn.
    pub async fn spawn() -> MyActorSender<()> {
        use futures::future::FutureExt;

        let (sender, driver) = MyActorSender::ghost_actor_spawn(Box::new(|_| {
            async move {
                Ok(MyActorImpl)
            }.boxed().into()
        })).await.unwrap();

        tokio::task::spawn(driver);

        sender
    }
}

async fn async_main() {
    let mut sender = MyActorImpl::spawn().await;

    assert_eq!(43, sender.add_one(42).await.unwrap());

    sender.ghost_actor_shutdown().await.unwrap();

    assert_eq!(
        "Err(GhostError(SendError(SendError { kind: Disconnected })))",
        &format!("{:?}", sender.add_one(42).await),
    );
}
```
