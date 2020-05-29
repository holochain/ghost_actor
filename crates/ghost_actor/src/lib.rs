#![deny(warnings)]
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
//! # use ghost_actor::dependencies::futures::future::FutureExt;
//! #[derive(Debug, thiserror::Error)]
//! pub enum MyError {
//!     #[error(transparent)]
//!     GhostError(#[from] ghost_actor::GhostError),
//! }
//!
//! ghost_actor::ghost_actor! {
//!     // Set the visibility of your actor.
//!     // Name your actor.
//!     // Specify the Error type for your actor.
//!     // The error type must implement `From<GhostError>`.
//!
//!     /// Api Docs that should appear on the Sender type for your actor.
//!     pub actor MyActor<MyError> {
//!         // specify your actor api
//!
//!         /// This string will be applied as docs to sender/handler.
//!         fn add_one(
//!             // any api params here
//!             input: u32,
//!         ) -> u32; // return type here
//!     }
//! }
//!
//! /// An example implementation of the example MyActor GhostActor.
//! struct MyActorImpl;
//!
//! // The generics for a handler are:
//! // 1 - the "custom" type you'd like to allow users of your api to send in.
//! // 2 - the "internal" type you'd like your handlers to send in.
//! // It is highly recommended to use a `ghost_chan!` type for these.
//! // However, if you have no use for these capabilities, use `()`.
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
//!         let (sender, driver) = MyActorSender::ghost_actor_spawn(Box::new(|_internal_sender| {
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
//!     let res = format!("{:?}", sender.add_one(42).await);
//!     if &res != "Err(GhostError(SendError(SendError { kind: Disconnected })))"
//!         && &res != "Err(GhostError(ResponseError(Canceled)))"
//!     {
//!         panic!("expected send error");
//!     }
//! }
//! ```
//!
//! # Implementing a Handler
//!
//! The `ghost_actor!` macro is going to generate a "[Name]Handler" trait.
//! To provide an implementation for your `ghost_actor!` type, you need an
//! item that implements this trait (see example above).
//!
//! In addition to all the `handle_*` methods that are auto-generated per
//! the `Api` section in the macro, there are also provided implementations
//! for `handle_ghost_actor_custom` and `handle_ghost_actor_internal`.
//!
//! Please see any of the unit tests (or run `cargo doc` on a module containing
//! your `ghost_actor!` macro invocation) for examples on how to implement
//! a handler.
//!
//! # Implementing a Spawn function
//!
//! While you can absolutely require users of your api to call
//! `YourTypeSender::ghost_actor_spawn(...)` and instantiate your handler type
//! inside the callback, it might be polite to provide a function that requires
//! a little less boilerplate.
//!
//! See the example above, however, there may be no need to expose the
//! implemented item type at all, you could, for example:
//!
//! ```
//! # #[derive(Debug, thiserror::Error)]
//! # pub enum MyError {
//! #     #[error(transparent)]
//! #     GhostError(#[from] ghost_actor::GhostError),
//! # }
//! # ghost_actor::ghost_actor! {
//! #     pub actor MyActor<MyError> {
//! #         fn add_one(
//! #             input: u32,
//! #         ) -> u32;
//! #     }
//! # }
//! /// internal private type
//! struct MyActorImpl;
//!
//! impl MyActorHandler<(), ()> for MyActorImpl {
//!     // ...
//! #    fn handle_add_one(&mut self, input: u32) -> MyActorHandlerResult<u32> {
//! #        unimplemented!();
//! #    }
//! }
//!
//! /// Rather than using ghost_actor_spawn directly, use this simple spawn.
//! /// This spawn makes an assumption that we are in a tokio runtime,
//! /// if we don't want to make that assumption, we can also return the
//! /// driver future here.
//! pub async fn spawn_my_actor() -> MyActorSender {
//!     use futures::future::FutureExt;
//!
//!     let (sender, driver) = MyActorSender::ghost_actor_spawn(Box::new(|_internal_sender| {
//!         async move {
//!             Ok(MyActorImpl)
//!         }.boxed().into()
//!     })).await.unwrap();
//!
//!     tokio::task::spawn(driver);
//!
//!     sender
//! }
//! # #[tokio::main(threaded_scheduler)]
//! # async fn main() {
//! # }
//! ```
//!
//! # The `ghost_chan!` macro.
//!
//! The `ghost_chan!` macro has an identical API to the `ghost_actor!` macro.
//! And, in fact, the `ghost_actor!` macro invokes `ghost_chan!` to produce
//! an internal enum for sending messages from your `Sender` struct.
//!
//! When implementing a ghost actor Handler that will make use of Custom
//! and/or Internal types, it is recommended to use a `ghost_chan!` enum as
//! this type.
//!
//! See the unit/integration tests for examples on making use of these.

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

mod r#macro;
pub use r#macro::*;

mod tests;
