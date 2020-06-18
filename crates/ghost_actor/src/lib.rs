#![deny(warnings)]
#![deny(missing_docs)]
#![allow(clippy::needless_doctest_main)]
#![cfg_attr(feature = "unstable", feature(fn_traits))]
#![cfg_attr(feature = "unstable", feature(unboxed_closures))]
//! A simple, ergonomic, idiomatic, macro for generating the boilerplate
//! to use rust futures tasks in a concurrent actor style.
//!
//! ## Hello World Example
//!
//! ```rust
//! # use ghost_actor::*;
//! // Most of the GhostActor magic happens in this macro.
//! // Sender and Handler traits will be generated here.
//! ghost_actor! {
//!     pub actor HelloWorldActor<GhostError> {
//!         fn hello_world() -> String;
//!     }
//! }
//!
//! // ... We'll skip implementing a handler for now ...
//! # struct HelloWorldImpl;
//! # impl GhostControlHandler for HelloWorldImpl {}
//! # impl GhostHandler<HelloWorldActor> for HelloWorldImpl {}
//! # impl HelloWorldActorHandler for HelloWorldImpl {
//! #     fn handle_hello_world(
//! #         &mut self,
//! #     ) -> HelloWorldActorHandlerResult<String> {
//! #         Ok(must_future::MustBoxFuture::new(async move {
//! #             Ok("hello world!".to_string())
//! #         }))
//! #     }
//! # }
//! # impl HelloWorldImpl {
//! #     pub async fn spawn() -> GhostSender<HelloWorldActor> {
//! #         let builder = actor_builder::GhostActorBuilder::new();
//! #         let sender = builder
//! #             .channel_factory()
//! #             .create_channel::<HelloWorldActor>()
//! #             .await
//! #             .unwrap();
//! #         tokio::task::spawn(builder.spawn(HelloWorldImpl));
//! #         sender
//! #     }
//! # }
//!
//! #[tokio::main]
//! async fn main() {
//!     // spawn our actor, getting the actor sender.
//!     let sender = HelloWorldImpl::spawn().await;
//!
//!     // we can make async calls on the sender
//!     assert_eq!("hello world!", &sender.hello_world().await.unwrap());
//! }
//! ```
//!
//! What's going on Here?
//!
//! - The `ghost_actor!` macro writes some types and boilerplate for us.
//! - We'll dig into implementing actor handlers below.
//! - We are able to spawn an actor that runs as a futures task.
//! - We can make async requests on that actor, and get results inline.
//!
//! ## The `ghost_actor!` Macro
//!
//! ```rust
//! # use ghost_actor::*;
//! ghost_actor! {
//!     pub actor HelloWorldActor<GhostError> {
//!         fn hello_world() -> String;
//!     }
//! }
//! # pub fn main() {}
//! ```
//!
//! The `ghost_actor!` macro takes care of writing the boilerplate for using
//! async functions to communicate with an "actor" running as a futures
//! task. The tests/examples here use tokio for the task executor, but
//! the GhostActorBuilder returns a driver future for the actor task that you
//! can manage any way you'd like.
//!
//! The `ghost_actor!` macro generates some important types, many of which
//! are derived by pasting words on to the end of your actor name.
//! We'll use the actor name `HelloWorldActor` from above as an example:
//!
//! - `HelloWorldActorSender` - The "Sender" trait generated for your actor
//!   allows users with a `GhostSender<HelloWorldActor>` instance to make
//!   async calls. Basically, this "Sender" trait provides the API that
//!   makes the whole actor system work.
//! - `HelloWorldActorHandler` - This "Handler" trait is what allows you
//!   to implement an actor task that can respond to requests sent by
//!   the "Sender".
//! - `HelloWorldActor` - You may have noticed above, the "Sender" instance
//!   that users of your api will receive is typed as
//!   `GhostSender<HelloWorldActor>`. The item that receives the name of your
//!   actor without having anything pasted on to it is actually a `GhostEvent`
//!   enum designed for carrying messages from your "Sender" to your
//!   "Handler", and then delivering the result back to your API user.
//!
//! ## Implementing an Actor Handler
//!
//! ```rust
//! # use ghost_actor::*;
//! # ghost_actor! {
//! #     pub actor HelloWorldActor<GhostError> {
//! #         fn hello_world() -> String;
//! #     }
//! # }
//! // We need a struct to implement our handler upon.
//! struct HelloWorldImpl;
//!
//! // All handlers must implement GhostControlHandler.
//! // This provides a default no-op handle_ghost_actor_shutdown impl.
//! impl GhostControlHandler for HelloWorldImpl {}
//!
//! // Implement GhostHandler for your specific GhostEvent type.
//! // Don't worry, the compiler will let you know if you forget this : )
//! impl GhostHandler<HelloWorldActor> for HelloWorldImpl {}
//!
//! // Now implement your actual handler -
//! // auto generated by the `ghost_event!` macro.
//! impl HelloWorldActorHandler for HelloWorldImpl {
//!     fn handle_hello_world(
//!         &mut self,
//!     ) -> HelloWorldActorHandlerResult<String> {
//!         Ok(must_future::MustBoxFuture::new(async move {
//!             // return our results
//!             Ok("hello world!".to_string())
//!         }))
//!     }
//! }
//! # pub fn main() {}
//! ```
//!
//! Pretty straight forward. We implement a couple required traits,
//! then our "Handler" trait that actually defines the logic of our actor.
//! Then, we're ready to spawn it!
//!
//! ## Spawning an actor
//!
//! ```rust
//! # use ghost_actor::*;
//! # ghost_actor! {
//! #     pub actor HelloWorldActor<GhostError> {
//! #         fn hello_world() -> String;
//! #     }
//! # }
//! # struct HelloWorldImpl;
//! # impl GhostControlHandler for HelloWorldImpl {}
//! # impl GhostHandler<HelloWorldActor> for HelloWorldImpl {}
//! # impl HelloWorldActorHandler for HelloWorldImpl {
//! #     fn handle_hello_world(
//! #         &mut self,
//! #     ) -> HelloWorldActorHandlerResult<String> {
//! #         Ok(must_future::MustBoxFuture::new(async move {
//! #             Ok("hello world!".to_string())
//! #         }))
//! #     }
//! # }
//! impl HelloWorldImpl {
//!     // Use the GhostActorBuilder to construct the actor task.
//!     pub async fn spawn() -> GhostSender<HelloWorldActor> {
//!         // first we need a builder
//!         let builder = actor_builder::GhostActorBuilder::new();
//!
//!         // now let's register an event channel with this actor.
//!         let sender = builder
//!             .channel_factory()
//!             .create_channel::<HelloWorldActor>()
//!             .await
//!             .unwrap();
//!
//!         // actually spawn the actor driver task
//!         // providing our implementation
//!         tokio::task::spawn(builder.spawn(HelloWorldImpl));
//!
//!         // return the sender that controls the actor
//!         sender
//!     }
//! }
//! # pub fn main() {}
//! ```
//!
//! Note how we actually get access to the cheaply-clonable "Sender"
//! before we have to construct our actor "Handler" item. This means
//! you can create channels that will be able to message the actor,
//! and include those senders in your handler struct. More on this later.
//!
//! ## The Complete Hello World Example
//!
//! ```rust
//! # use ghost_actor::*;
//! // Most of the GhostActor magic happens in this macro.
//! // Sender and Handler traits will be generated here.
//! ghost_actor! {
//!     pub actor HelloWorldActor<GhostError> {
//!         fn hello_world() -> String;
//!     }
//! }
//!
//! // We need a struct to implement our handler upon.
//! struct HelloWorldImpl;
//!
//! // All handlers must implement GhostControlHandler.
//! // This provides a default no-op handle_ghost_actor_shutdown impl.
//! impl GhostControlHandler for HelloWorldImpl {}
//!
//! // Implement GhostHandler for your specific GhostEvent type.
//! // Don't worry, the compiler will let you know if you forget this : )
//! impl GhostHandler<HelloWorldActor> for HelloWorldImpl {}
//!
//! // Now implement your actual handler -
//! // auto generated by the `ghost_event!` macro.
//! impl HelloWorldActorHandler for HelloWorldImpl {
//!     fn handle_hello_world(
//!         &mut self,
//!     ) -> HelloWorldActorHandlerResult<String> {
//!         Ok(must_future::MustBoxFuture::new(async move {
//!             // return our results
//!             Ok("hello world!".to_string())
//!         }))
//!     }
//! }
//!
//! impl HelloWorldImpl {
//!     // Use the GhostActorBuilder to construct the actor task.
//!     pub async fn spawn() -> GhostSender<HelloWorldActor> {
//!         // first we need a builder
//!         let builder = actor_builder::GhostActorBuilder::new();
//!
//!         // now let's register an event channel with this actor.
//!         let sender = builder
//!             .channel_factory()
//!             .create_channel::<HelloWorldActor>()
//!             .await
//!             .unwrap();
//!
//!         // actually spawn the actor driver task
//!         // providing our implementation
//!         tokio::task::spawn(builder.spawn(HelloWorldImpl));
//!
//!         // return the sender that controls the actor
//!         sender
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     // spawn our actor, getting the actor sender.
//!     let sender = HelloWorldImpl::spawn().await;
//!
//!     // we can make async calls on the sender
//!     assert_eq!("hello world!", &sender.hello_world().await.unwrap());
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

mod actor_macro;
pub use actor_macro::*;

pub mod actor_builder;

mod tests;
