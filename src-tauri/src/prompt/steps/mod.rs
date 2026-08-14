// Atlas OS — Pipeline step modules (RFC 23 §2). Each step is a pure
// transformation; the runner in `super::runner` threads them together.

pub mod capture;
pub mod clarify;
pub mod consolidate;
pub mod detect;
pub mod parse;
pub mod similar;
