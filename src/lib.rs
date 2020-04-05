#![deny(missing_docs)]
#![allow(clippy::needless_doctest_main)]
//! A simple, ergonomic, idiomatic, macro for generating the boilerplate to use rust futures tasks in a concurrent actor style.
//!
//! # Example
//!
//! ```
//! # use ghost_actor::example::MyError;
//! ghost_actor::ghost_actor! {
//!     name: pub MyActor,
//!     error: MyError,
//!     api: {
//!         AddOne::add_one(
//!             "A test function, output adds 1 to input.",
//!             u32, u32),
//!     }
//! }
//!
//! /// An example implementation of the example MyActor GhostActor.
//! struct MyActorImpl;
//!
//! impl MyActorHandler<(), ()> for MyActorImpl {
//!     fn handle_add_one(
//!         &mut self,
//!         _: &mut MyActorInternalSender<(), ()>,
//!         input: u32,
//!     ) -> Result<u32, MyError> {
//!         Ok(input + 1)
//!     }
//! }
//!
//! impl MyActorImpl {
//!     /// Rather than using ghost_actor_spawn directly, use this simple spawn.
//!     pub fn spawn() -> MyActorSender<()> {
//!         let (sender, driver) = MyActorSender::ghost_actor_spawn(MyActorImpl);
//!         tokio::task::spawn(driver);
//!         sender
//!     }
//! }
//!
//! async fn async_main() {
//!     let mut sender = MyActorImpl::spawn();
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
//! # pub fn main() {
//! #     tokio::runtime::Builder::new()
//! #         .threaded_scheduler()
//! #         .build().unwrap().block_on(async_main());
//! # }
//! ```

mod types;
pub use types::*;

mod ghost_chan;
pub use ghost_chan::*;

mod macros;
pub use macros::*;

mod tests;
pub use tests::*;
