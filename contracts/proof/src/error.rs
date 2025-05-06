use ckb_std::error::SysError;
use common::error::Error as CommonError;

/// Error
#[repr(i8)]
pub enum Error {
    IndexOutOfBound = 1,
    ItemMissing,
    LengthNotEnough,
    Encoding,

    // common error
    InvalidArgs,
    InvalidCampaignId,
    InvalidEntityId,
    InvalidHash,
    InvalidProof,
    InvalidPublicKey,
    InvalidSignature,
    InvalidTimestamp,
    InvalidTypeId,
    InvalidUserLock,

    // self error
    CellNotFound,
    NoInputCells,
    TransferNotAllowed,
    VerificationWitnessNotFound,
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

impl From<CommonError> for Error {
    fn from(err: CommonError) -> Self {
        use CommonError::*;
        match err {
            InvalidCampaignId => Self::InvalidCampaignId,
            InvalidEntityId => Self::InvalidEntityId,
            InvalidHash => Self::InvalidHash,
            InvalidProof => Self::InvalidProof,
            InvalidPublicKey => Self::InvalidPublicKey,
            InvalidSignature => Self::InvalidSignature,
            InvalidTimestamp => Self::InvalidTimestamp,
            InvalidUserLock => Self::InvalidUserLock,
            ItemMissing => Self::ItemMissing,
            _ => panic!("unexpected common error"),
        }
    }
}
