#![no_std]

extern crate alloc;

pub mod error;
pub mod generated;
pub mod merkle;
pub mod type_id;
pub mod utils;
mod validation;

pub use validation::validate_proof_against_witness;
