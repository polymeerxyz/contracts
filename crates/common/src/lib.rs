#![no_std]

extern crate alloc;

pub const NULL_HASH: [u8; 32] = [0u8; 32];

pub mod conversion;
pub mod error;
pub mod utils;

#[cfg(feature = "type_id")]
pub mod type_id;

pub mod base {
    include!(concat!(env!("OUT_DIR"), "/base.rs"));
}

pub mod schema {
    #[cfg(feature = "distribution")]
    include!(concat!(env!("OUT_DIR"), "/distribution.rs"));

    #[cfg(feature = "proof")]
    include!(concat!(env!("OUT_DIR"), "/proof.rs"));

    #[cfg(feature = "vault")]
    include!(concat!(env!("OUT_DIR"), "/vault.rs"));
}
