#![no_std]

extern crate alloc;

pub mod error;
pub mod generated;
pub mod merkle;
mod validation;

pub use validation::validate_proof_against_witness;
