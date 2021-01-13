#![forbid(unsafe_code)]
#![forbid(warnings)]
#![forbid(missing_docs)]
//! GhostActor makes it simple, ergonomic, and idiomatic to implement
//! async / concurrent code using an Actor model.
//!
//! GhostActor uses only safe code, and is futures executor agnostic--use
//! tokio, futures, async-std, whatever you want. The following examples use
//! tokio.
//!
//! # What does it do?
//!
//! The GhostActor struct is a `'static + Send + Sync` cheaply clone-able
//! handle for managing rapid, efficient, sequential, mutable access to
//! internal state data.
//!
//! # Using the raw type:
//!
//! ```
//! # use ghost_actor::*;
//! # #[tokio::main]
//! # async fn main() {
//! // set our initial state
//! let (a, driver) = GhostActor::new(42_u32);
//!
//! // spawn the driver--using tokio here as an example
//! tokio::task::spawn(driver);
//!
//! // invoke some logic on the internal state (just reading here)
//! let result: Result<u32, GhostError> = a.invoke(|a| Ok(*a)).await;
//!
//! // assert the result
//! assert_eq!(42, result.unwrap());
//! # }
//! ```
//!
//! # Best Practice: Internal state in a New Type:
//!
//! GhostActor is easiest to work with when you have an internal state struct,
//! wrapped in a new type of a GhostActor:
//!
//! ```
//! # use ghost_actor::*;
//! # #[tokio::main]
//! # async fn main() {
//! struct InnerState {
//!     age: u32,
//!     name: String,
//! }
//!
//! #[derive(Clone, PartialEq, Eq, Hash)]
//! pub struct Person(GhostActor<InnerState>);
//!
//! impl Person {
//!     pub fn new(age: u32, name: String) -> Self {
//!         let (actor, driver) = GhostActor::new(InnerState { age, name });
//!         tokio::task::spawn(driver);
//!         Self(actor)
//!     }
//!
//!     pub async fn birthday(&self) -> String {
//!         self.0.invoke(|inner| {
//!             inner.age += 1;
//!             let msg = format!(
//!                 "Happy birthday {}, you are {} years old.",
//!                 inner.name,
//!                 inner.age,
//!             );
//!             <Result::<String, GhostError>>::Ok(msg)
//!         }).await.unwrap()
//!     }
//! }
//!
//! let bob = Person::new(42, "Bob".to_string());
//! assert_eq!(
//!     "Happy birthday Bob, you are 43 years old.",
//!     &bob.birthday().await,
//! );
//! # }
//! ```
//!
//! # Using traits (and GhostFuture) to provide dynamic actor types:
//!
//! ```
//! # use ghost_actor::*;
//! # #[tokio::main]
//! # async fn main() {
//! pub trait Fruit {
//!     // until async traits are available in rust, you can use GhostFuture
//!     fn eat(&self) -> GhostFuture<String, GhostError>;
//!
//!     // allows implementing clone on BoxFruit
//!     fn box_clone(&self) -> BoxFruit;
//! }
//!
//! pub type BoxFruit = Box<dyn Fruit>;
//!
//! impl Clone for BoxFruit {
//!     fn clone(&self) -> Self {
//!         self.box_clone()
//!     }
//! }
//!
//! #[derive(Clone, PartialEq, Eq, Hash)]
//! pub struct Banana(GhostActor<u32>);
//!
//! impl Banana {
//!     pub fn new() -> BoxFruit {
//!         let (actor, driver) = GhostActor::new(0);
//!         tokio::task::spawn(driver);
//!         Box::new(Self(actor))
//!     }
//! }
//!
//! impl Fruit for Banana {
//!     fn eat(&self) -> GhostFuture<String, GhostError> {
//!         let fut = self.0.invoke(|count| {
//!             *count += 1;
//!             <Result<u32, GhostError>>::Ok(*count)
//!         });
//!
//!         // 'resp()' is a helper function that builds a GhostFuture
//!         // from any other future that has a matching Output.
//!         resp(async move {
//!             Ok(format!("ate {} bananas", fut.await.unwrap()))
//!         })
//!     }
//!
//!     fn box_clone(&self) -> BoxFruit {
//!         Box::new(self.clone())
//!     }
//! }
//!
//! // we could implement a similar 'Apple' struct
//! // that could be interchanged here:
//! let fruit: BoxFruit = Banana::new();
//! assert_eq!("ate 1 bananas", &fruit.eat().await.unwrap());
//! # }
//! ```
//!
//! # Custom GhostActor error types:
//!
//! The `GhostActor::invoke()` function takes a generic error type.
//! The only requirement is that it must implement `From<GhostError>`:
//!
//! ```
//! # use ghost_actor::*;
//! # #[tokio::main]
//! # async fn main() {
//! #[derive(Debug)]
//! struct MyError;
//! impl std::error::Error for MyError {}
//! impl std::fmt::Display for MyError {
//!     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//!         write!(f, "{:?}", self)
//!     }
//! }
//! impl From<GhostError> for MyError {
//!     fn from(_: GhostError) -> Self {
//!         Self
//!     }
//! }
//!
//! let (actor, driver) = GhostActor::new(42_u32);
//! tokio::task::spawn(driver);
//! assert_eq!(42, actor.invoke(|inner| {
//!     <Result<u32, MyError>>::Ok(*inner)
//! }).await.unwrap());
//! # }
//! ```
//!
//! # Code Examples:
//!
//! - [Bounce](https://github.com/holochain/ghost_actor/blob/master/examples/bounce.rs): `cargo run --example bounce`
//!
//! # Contributing:
//!
//! This repo uses `cargo-task`.
//!
//! ```ignore
//! cargo install cargo-task
//! cargo task
//! ```

/// Re-exported dependencies.
pub mod dependencies {
    pub use futures;
    pub use tracing;
}

mod error;
pub use error::*;
mod as_ghost_actor;
use as_ghost_actor::ghost_actor_trait::*;
pub use as_ghost_actor::*;
mod driver;
pub use driver::*;
mod future;
pub use future::*;
mod config;
pub use config::*;
mod actor;
pub use actor::*;

#[cfg(test)]
mod test;
