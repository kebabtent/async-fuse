//! [![Documentation](https://docs.rs/async-fuse/badge.svg)](https://docs.rs/async-fuse)
//! [![Crates](https://img.shields.io/crates/v/async-fuse.svg)](https://crates.io/crates/async-fuse)
//! [![Actions Status](https://github.com/udoprog/async-fuse/workflows/Rust/badge.svg)](https://github.com/udoprog/async-fuse/actions)
//!
//! Helpers for fusing asynchronous computations.
//!
//! This is especially useful in combination with optional branches using
//! [tokio::select], where the future being polled isn't necessarily set. So
//! instead of requiring a potentially problematic [branch precondition], the
//! future will simply be marked as pending indefinitely.
//!
//! # Examples
//!
//! > This is available as the `ticker` example:
//! > ```sh
//! > cargo run --example ticker
//! > ```
//!
//! ```rust
//! use std::time::Duration;
//! use tokio::time;
//!
//! # #[tokio::main]
//! # async fn main() {
//! let sleep = async_fuse::Stack::new(time::sleep(Duration::from_millis(100)));
//! tokio::pin!(sleep);
//!
//! for _ in 0..20usize {
//!     (&mut sleep).await;
//!     assert!(sleep.is_empty());
//!
//!     println!("tick");
//!
//!     sleep.set(async_fuse::Stack::new(time::sleep(Duration::from_millis(100))))
//! }
//! # }
//! ```
//!
//! [tokio::select]: https://docs.rs/tokio/1/tokio/macro.select.html
//! [branch precondition]: https://docs.rs/tokio/1.0.1/tokio/macro.select.html#avoid-racy-if-preconditions

#![deny(missing_docs)]

mod heap;
mod stack;

pub use self::heap::Heap;
pub use self::stack::Stack;
