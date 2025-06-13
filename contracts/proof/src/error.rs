use ckb_std::error::SysError;
use common::error::Error as CommonError;

#[repr(i8)]
pub enum Error {
    IndexOutOfBound = 1,
    ItemMissing,
    LengthNotEnough,
    Encoding,

    InvalidProofDataStructure = 100,
    InvalidProofCellRecreation,
    InvalidProofTxStructure,
    InvalidSubscriberLock,

    External(i8),
}

impl From<SysError> for Error {
    fn from(err: SysError) -> Self {
        match err {
            SysError::IndexOutOfBound => Self::IndexOutOfBound,
            SysError::ItemMissing => Self::ItemMissing,
            SysError::LengthNotEnough(_) => Self::LengthNotEnough,
            SysError::Encoding => Self::Encoding,
            _ => panic!("unexpected sys error"),
        }
    }
}

impl From<CommonError> for Error {
    fn from(err: CommonError) -> Self {
        Self::External(err as i8)
    }
}

impl From<Error> for i8 {
    fn from(err: Error) -> i8 {
        match err {
            Error::External(v) => v,
            _ => i8::from(err),
        }
    }
}
