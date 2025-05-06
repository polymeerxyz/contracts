use ckb_std::error::SysError;

/// Error
#[repr(i8)]
pub enum Error {
    IndexOutOfBound = 1,
    ItemMissing,
    LengthNotEnough,
    Encoding,

    InvalidCampaignId,
    InvalidEntityId,
    InvalidHash,
    InvalidProof,
    InvalidPublicKey,
    InvalidSignature,
    InvalidTimestamp,
    InvalidUserLock,
}

impl From<SysError> for Error {
    fn from(err: SysError) -> Self {
        use SysError::*;
        match err {
            IndexOutOfBound => Self::IndexOutOfBound,
            ItemMissing => Self::ItemMissing,
            LengthNotEnough(_) => Self::LengthNotEnough,
            Encoding => Self::Encoding,
            _ => panic!("unexpected sys error"),
        }
    }
}
