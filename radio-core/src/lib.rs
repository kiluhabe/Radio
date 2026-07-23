#![cfg_attr(not(test), no_std)]

pub mod types;
pub use types::*;

pub mod quadrature;
pub use quadrature::QuadratureDecoder;
