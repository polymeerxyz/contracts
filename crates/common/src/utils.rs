use alloc::{string::ToString, vec, vec::Vec};
use faster_hex::hex_decode;
use molecule::{error::VerificationError, prelude::Entity};

pub fn decode_hex<T>(val: &str) -> Result<T, VerificationError>
where
    T: Entity,
{
    let raw = val.as_bytes();
    let len = raw.len() / 2;
    let mut dst = vec![0; len];
    let _ = hex_decode(raw, &mut dst)
        .map_err(|_| VerificationError::OffsetsNotMatch(T::NAME.to_string()));
    T::from_slice(&dst)
}

pub fn hex_to_vec(val: &str) -> Vec<u8> {
    let raw = val.as_bytes();
    let mut dst = vec![0; raw.len() / 2];
    let _ = hex_decode(raw, &mut dst);
    dst
}
