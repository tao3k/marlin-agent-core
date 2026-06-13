//! Build-script support for the native Deck runtime integration crate.

mod discovery;
mod script;

pub use script::{emit_deck_runtime_native_link_directives, rustc_wrapper_is_clippy};
