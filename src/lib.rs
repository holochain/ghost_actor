#![deny(missing_docs)]
#![allow(clippy::needless_doctest_main)]
//! A simple, ergonomic, idiomatic, macro for generating the boilerplate to use rust futures tasks in a concurrent actor style.
//!
//! # What is GhostActor?
//!
//! GhostActor boils down to a macro that helps you write all the boilerplate
//! needed to treat a Future like an actor. When you "spawn" a GhostActor,
//! you receive a handle called a "Sender", that allows you to make async
//! requests and inline await async responses to/from you actor implementation's
//! driver task.
//!
//! The senders are cheaply clone-able allowing you to easily execute any
//! number of parallel workflows with your task. When all senders are dropped,
//! or if you explicitly call `ghost_actor_shutdown()`, the driver task
//! (a.k.a. your Actor) will end.
//!
//! # Example
//!
//! ```
//! # use ghost_actor::example::MyError;
//! # use ghost_actor::dependencies::futures::future::FutureExt;
//! ghost_actor::ghost_actor! {
//!     // set visibility and name your actor
//!     name: pub MyActor,
//!
//!     // any custom error set here must implement `From<GhostError>`
//!     error: MyError,
//!
//!     // specify your actor api
//!     api: {
//!         // someday if the `paste` crate supported inflection
//!         // we won't have to specify both inflections here.
//!         AddOne::add_one(
//!             // this string will be applied as docs to sender/handler
//!             "A test function, output adds 1 to input.",
//!
//!             // the input type for your api
//!             u32,
//!
//!             // the output type for your api
//!             u32
//!         ),
//!     }
//! }
//!
//! /// An example implementation of the example MyActor GhostActor.
//! struct MyActorImpl;
//!
//! impl MyActorHandler<(), ()> for MyActorImpl {
//!     fn handle_add_one(
//!         &mut self,
//!         input: u32,
//!     ) -> MyActorHandlerResult<u32> {
//!         Ok(async move {
//!             Ok(input + 1)
//!         }.boxed().into())
//!     }
//! }
//!
//! impl MyActorImpl {
//!     /// Rather than using ghost_actor_spawn directly, use this simple spawn.
//!     pub async fn spawn() -> MyActorSender {
//!         use futures::future::FutureExt;
//!
//!         let (sender, driver) = MyActorSender::ghost_actor_spawn(Box::new(|_| {
//!             async move {
//!                 Ok(MyActorImpl)
//!             }.boxed().into()
//!         })).await.unwrap();
//!
//!         tokio::task::spawn(driver);
//!
//!         sender
//!     }
//! }
//!
//! #[tokio::main(threaded_scheduler)]
//! async fn main() {
//!     let mut sender = MyActorImpl::spawn().await;
//!
//!     assert_eq!(43, sender.add_one(42).await.unwrap());
//!
//!     sender.ghost_actor_shutdown().await.unwrap();
//!
//!     assert_eq!(
//!         "Err(GhostError(SendError(SendError { kind: Disconnected })))",
//!         &format!("{:?}", sender.add_one(42).await),
//!     );
//! }
//! ```

/// Re-exported dependencies to help with macro references.
pub mod dependencies {
    pub use futures;
    pub use must_future;
    pub use paste;
    pub use thiserror;
    pub use tracing;
}

mod types;
pub use types::*;

pub mod ghost_chan;

mod macros;
pub use macros::*;

mod tests;
pub use tests::*;
