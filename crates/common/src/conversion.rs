use ckb_std::ckb_types::prelude::{Pack, Unpack};
use molecule::{
    bytes::Bytes,
    prelude::{Entity, Reader},
};

use crate::base::{Uint16, Uint16Reader, Uint32, Uint32Reader, Uint64, Uint64Reader};

impl Pack<Uint16> for u16 {
    fn pack(&self) -> Uint16 {
        Uint16::new_unchecked(Bytes::from(self.to_le_bytes().to_vec()))
    }
}

impl Unpack<u16> for Uint16Reader<'_> {
    fn unpack(&self) -> u16 {
        let mut b = [0u8; 2];
        b.copy_from_slice(self.as_slice());
        u16::from_le_bytes(b)
    }
}

impl Unpack<u16> for Uint16 {
    fn unpack(&self) -> u16 {
        self.as_reader().unpack()
    }
}

impl Pack<Uint32> for u32 {
    fn pack(&self) -> Uint32 {
        Uint32::new_unchecked(Bytes::from(self.to_le_bytes().to_vec()))
    }
}

impl Unpack<u32> for Uint32Reader<'_> {
    fn unpack(&self) -> u32 {
        let mut b = [0u8; 4];
        b.copy_from_slice(self.as_slice());
        u32::from_le_bytes(b)
    }
}

impl Unpack<u32> for Uint32 {
    fn unpack(&self) -> u32 {
        self.as_reader().unpack()
    }
}

impl Pack<Uint64> for u64 {
    fn pack(&self) -> Uint64 {
        Uint64::new_unchecked(Bytes::from(self.to_le_bytes().to_vec()))
    }
}

impl Unpack<u64> for Uint64Reader<'_> {
    fn unpack(&self) -> u64 {
        let mut b = [0u8; 8];
        b.copy_from_slice(self.as_slice());
        u64::from_le_bytes(b)
    }
}

impl Unpack<u64> for Uint64 {
    fn unpack(&self) -> u64 {
        self.as_reader().unpack()
    }
}
