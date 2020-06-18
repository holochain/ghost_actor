#![deny(warnings)]
#![deny(missing_docs)]
#![allow(clippy::needless_doctest_main)]
#![cfg_attr(feature = "unstable", feature(fn_traits))]
#![cfg_attr(feature = "unstable", feature(unboxed_closures))]

//! A simple, ergonomic, idiomatic, macro for generating the boilerplate
//! to use rust futures tasks in a concurrent actor style.

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

mod event_macro;
pub use event_macro::*;

pub mod actor_builder;

mod tests;
