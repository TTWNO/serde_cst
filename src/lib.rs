//! A way to (de)serialize CST (Carnegie Speech Tools) files.
//! The most common type are `.flitevox` files used for the [flite TTS engine](https://github.com/festvox/flite).
//!
//! It is not recommended to use this crate without the types provided (must enable the `alloc` feature). The format is not well
//! suited to general use.
//!
//! This crate is `no_std` compatible, but `std` support can be activated if desired.
#[cfg(feature = "alloc")]
pub mod date;
pub mod de;
pub mod error;
pub mod gender;
pub mod ser;
pub use gender::*;
#[cfg(feature = "alloc")]
pub mod header;
#[cfg(feature = "alloc")]
pub use header::*;

#[cfg(feature = "std")]
extern crate std;
