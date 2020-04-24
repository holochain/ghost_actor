![Crates.io](https://img.shields.io/crates/l/ghost_actor)
![Crates.io](https://img.shields.io/crates/v/ghost_actor)

# ghost_actor

A simple, ergonomic, idiomatic, macro for generating the boilerplate to use rust futures tasks in a concurrent actor style.

## What is GhostActor?

GhostActor boils down to a macro that helps you write all the boilerplate
needed to treat a Future like an actor. When you "spawn" a GhostActor,
you receive a handle called a "Sender", that allows you to make async
requests and inline await async responses to/from you actor implementation's
driver task.

The senders are cheaply clone-able allowing you to easily execute any
number of parallel workflows with your task. When all senders are dropped,
or if you explicitly call `ghost_actor_shutdown()`, the driver task
(a.k.a. your Actor) will end.

## Example

```rust
ghost_actor::ghost_actor! {
    // set the visibility of your actor - `()` for private.
    Visibility(pub),

    // name your actor
    Name(MyActor),

    // any custom error set here must implement `From<GhostError>`
    Error(MyError),

    // specify your actor api
    Api {
        // Method names will be transformed into snake_case,
        // so, this method will be called "add_one".
        AddOne(
            // this string will be applied as docs to sender/handler
            "A test function, output adds 1 to input.",

            // the input type for your api
            u32,

            // the output type for your api
            u32,
        ),
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
    pub async fn spawn() -> MyActorSender {
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

#[tokio::main(threaded_scheduler)]
async fn main() {
    let mut sender = MyActorImpl::spawn().await;

    assert_eq!(43, sender.add_one(42).await.unwrap());

    sender.ghost_actor_shutdown().await.unwrap();

    assert_eq!(
        "Err(GhostError(SendError(SendError { kind: Disconnected })))",
        &format!("{:?}", sender.add_one(42).await),
    );
}
```
