//! clio-custody — Hash incremental, cadeia de hashes, manifesto e selo local Ed25519.
#![forbid(unsafe_code)]
#![warn(clippy::all)]

pub mod chain;
pub mod error;
pub mod manifest;
pub mod seal;

pub use chain::{ChainBlock, HashChain};
pub use manifest::{Manifest, gerar_manifesto};
pub use seal::{LocalSeal, SealResult};
