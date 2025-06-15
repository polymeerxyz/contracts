#![no_std]

extern crate alloc;

pub const NULL_HASH: [u8; 32] = [0u8; 32];

pub mod conversion;
pub mod error;
pub mod utils;

#[cfg(feature = "type_id")]
pub mod type_id;

mod generated;

pub use generated::base;

pub mod schema {
    #![allow(clippy::all)]
    #![allow(unknown_lints)]
    #![allow(warnings)]

    #[cfg(feature = "distribution")]
    pub use crate::generated::distribution;

    #[cfg(feature = "proof")]
    pub use crate::generated::proof;

    #[cfg(feature = "vault")]
    pub use crate::generated::vault;
}
