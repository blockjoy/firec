//! Rust API to interact with firecracker.

#![forbid(unsafe_code)]
#![deny(missing_debug_implementations, nonstandard_style)]
#![warn(missing_docs, rustdoc::missing_doc_code_examples, unreachable_pub)]

pub mod config;
mod error;
mod machine;

pub use error::*;
pub use machine::*;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
