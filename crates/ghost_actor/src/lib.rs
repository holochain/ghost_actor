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
//! ghost_chan! {
//!     pub chan HelloWorldApi<GhostError> {
//!         fn hello_world() -> String;
//!     }
//! }
//!
//! // ... We'll skip implementing a handler for now ...
//! # struct HelloWorldImpl;
//! # impl GhostControlHandler for HelloWorldImpl {}
//! # impl GhostHandler<HelloWorldApi> for HelloWorldImpl {}
//! # impl HelloWorldApiHandler for HelloWorldImpl {
//! #     fn handle_hello_world(
//! #         &mut self,
//! #     ) -> HelloWorldApiHandlerResult<String> {
//! #         Ok(must_future::MustBoxFuture::new(async move {
//! #             Ok("hello world!".to_string())
//! #         }))
//! #     }
//! # }
//! # pub async fn spawn_hello_world(
//! # ) -> Result<GhostSender<HelloWorldApi>, GhostError> {
//! #     let builder = actor_builder::GhostActorBuilder::new();
//! #     let sender = builder
//! #         .channel_factory()
//! #         .create_channel::<HelloWorldApi>()
//! #         .await?;
//! #     tokio::task::spawn(builder.spawn(HelloWorldImpl));
//! #     Ok(sender)
//! # }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), GhostError> {
//!     // spawn our actor, getting the actor sender.
//!     let sender = spawn_hello_world().await?;
//!
//!     // we can make async calls on the sender
//!     assert_eq!("hello world!", &sender.hello_world().await?);
//!     println!("{}", sender.hello_world().await?);
//!
//!     Ok(())
//! }
//! ```
//!
//! What's going on Here?
//!
//! - The `ghost_chan!` macro writes some types and boilerplate for us.
//! - We'll dig into implementing actor handlers below.
//! - We are able to spawn an actor that runs as a futures task.
//! - We can make async requests on that actor, and get results inline.
//!
//! ## The `ghost_chan!` Macro
//!
//! ```rust
//! # use ghost_actor::*;
//! ghost_chan! {
//!     pub chan HelloWorldApi<GhostError> {
//!         fn hello_world() -> String;
//!     }
//! }
//! # pub fn main() {}
//! ```
//!
//! The `ghost_chan!` macro takes care of writing the boilerplate for using
//! async functions to communicate with an "actor" running as a futures
//! task. The tests/examples here use tokio for the task executor, but
//! the GhostActorBuilder returns a driver future for the actor task that you
//! can manage any way you'd like.
//!
//! The `ghost_chan!` macro generates some important types, many of which
//! are derived by pasting words on to the end of your actor name.
//! We'll use the actor name `HelloWorldApi` from above as an example:
//!
//! - `HelloWorldApiSender` - The "Sender" trait generated for your actor
//!   allows users with a `GhostSender<HelloWorldApi>` instance to make
//!   async calls. Basically, this "Sender" trait provides the API that
//!   makes the whole actor system work.
//! - `HelloWorldApiHandler` - This "Handler" trait is what allows you
//!   to implement an actor task that can respond to requests sent by
//!   the "Sender".
//! - `HelloWorldApi` - You may have noticed above, the "Sender" instance
//!   that users of your api will receive is typed as
//!   `GhostSender<HelloWorldApi>`. The item that receives the name of your
//!   actor without having anything pasted on to it is actually a `GhostEvent`
//!   enum designed for carrying messages from your "Sender" to your
//!   "Handler", and then delivering the result back to your API user.
//!
//! ## Implementing an Actor Handler
//!
//! ```rust
//! # use ghost_actor::*;
//! # ghost_chan! {
//! #     pub chan HelloWorldApi<GhostError> {
//! #         fn hello_world() -> String;
//! #     }
//! # }
//! /// We need a struct to implement our handler upon.
//! struct HelloWorldImpl;
//!
//! /// All handlers must implement GhostControlHandler.
//! /// This provides a default no-op handle_ghost_actor_shutdown impl.
//! impl GhostControlHandler for HelloWorldImpl {}
//!
//! /// Implement GhostHandler for your specific GhostEvent type.
//! /// Don't worry, the compiler will let you know if you forget this : )
//! impl GhostHandler<HelloWorldApi> for HelloWorldImpl {}
//!
//! /// Now implement your actual handler -
//! /// auto generated by the `ghost_chan!` macro.
//! impl HelloWorldApiHandler for HelloWorldImpl {
//!     fn handle_hello_world(&mut self) -> HelloWorldApiHandlerResult<String> {
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
//! ## Spawning an Actor
//!
//! ```rust
//! # use ghost_actor::*;
//! # ghost_chan! {
//! #     pub chan HelloWorldApi<GhostError> {
//! #         fn hello_world() -> String;
//! #     }
//! # }
//! # struct HelloWorldImpl;
//! # impl GhostControlHandler for HelloWorldImpl {}
//! # impl GhostHandler<HelloWorldApi> for HelloWorldImpl {}
//! # impl HelloWorldApiHandler for HelloWorldImpl {
//! #     fn handle_hello_world(
//! #         &mut self,
//! #     ) -> HelloWorldApiHandlerResult<String> {
//! #         Ok(must_future::MustBoxFuture::new(async move {
//! #             Ok("hello world!".to_string())
//! #         }))
//! #     }
//! # }
//! /// Use the GhostActorBuilder to construct the actor task.
//! pub async fn spawn_hello_world(
//! ) -> Result<GhostSender<HelloWorldApi>, GhostError> {
//!     // first we need a builder
//!     let builder = actor_builder::GhostActorBuilder::new();
//!
//!     // now let's register an event channel with this actor.
//!     let sender = builder
//!         .channel_factory()
//!         .create_channel::<HelloWorldApi>()
//!         .await?;
//!
//!     // actually spawn the actor driver task
//!     // providing our implementation
//!     tokio::task::spawn(builder.spawn(HelloWorldImpl));
//!
//!     // return the sender that controls the actor
//!     Ok(sender)
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
//! - [https://github.com/holochain/ghost_actor/blob/master/crates/ghost_actor/examples/hello_world.rs](https://github.com/holochain/ghost_actor/blob/master/crates/ghost_actor/examples/hello_world.rs)
//!
//! ## Custom Errors
//!
//! A single ghost channel / actor api will use a single error / result type.
//! You can use the provided `ghost_actor::GhostError` type - or you can
//! specify a custom error type.
//!
//! Your custom error type must support `From<GhostError>`.
//!
//! ```rust
//! # use ghost_actor::*;
//! #[derive(Debug, thiserror::Error)]
//! pub enum MyError {
//!     /// Custom error types MUST implement `From<GhostError>`
//!     #[error(transparent)]
//!     GhostError(#[from] GhostError),
//!
//!     /// Of course, you can also have your own variants as well
//!     #[error("My Error Type")]
//!     MyErrorType,
//! }
//!
//! ghost_chan! {
//!     /// The error type for actor apis is specified in the macro
//!     /// as the single generic following the actor name:
//!     pub chan MyActor<MyError> {
//!         fn my_fn() -> ();
//!     }
//! }
//! # pub fn main() {}
//! ```
//!
//! ## Efficiency! - Ghost Actor's Synchronous Handler Blocks
//!
//! GhostActor handler traits are carefully costructed to allow `&'a mut self`
//! access to the handler item, but return a `'static` future. That `'static`
//! means references to the handler item cannot be captured in any async code.
//!
//! This can be frustrating for new users, but serves a specific purpose!
//!
//! We are being good rust futures authors and working around any blocking
//! code in the manner our executor frameworks recommend, so our actor
//! handler can process messages at lightning speed!
//!
//! Our actor doesn't have to context switch, because it has all its mutable
//! internal state right here in this thread handling all these messages. And,
//! when it's done with one message, it moves right onto the next without
//! interuption. When the message queue is drained it schedules a wakeup for
//! when there is more data to process.
//!
//! In writing our code to support this pattern, we find that our code natually
//! tends toward patterns that support parallel work being done to make better
//! use of modern multi-core processors.
//!
//! See especially the "Internal Sender Pattern" in the next section below.
//!
//! ## Advanced Patterns for Working with Ghost Actors
//!
//! - [Internal Sender Pattern](https://github.com/holochain/ghost_actor/blob/master/crates/ghost_actor/examples/pattern_internal_sender.rs) -
//!   Facilitates undertaking async work in GhostActor handler functions.
//! - [Event Publish/Subscribe Pattern](https://github.com/holochain/ghost_actor/blob/master/crates/ghost_actor/examples/pattern_event_pub_sub.rs) -
//!   Facilitates an actor's ability to async emit notifications/requests,
//!   and a "parent" actor being able to handle events from a child actor.
//! - [Clone Channel Factory Pattern](https://github.com/holochain/ghost_actor/blob/master/crates/ghost_actor/examples/pattern_clone_channel_factory.rs) -
//!   Facilitates an actor's ability to absorb additional channel
//!   receivers post-spawn.

/// Re-exported dependencies to help with macro references.
pub mod dependencies {
    pub use futures;
    pub use must_future;
    pub use observability;
    pub use paste;
    pub use thiserror;
    pub use tracing;
    pub use tracing_futures;
}

mod types;
pub use types::*;

mod chan_macro;
pub use chan_macro::*;

pub mod actor_builder;

mod tests;
