use ckb_std::{debug, error::SysError};
use common::error::Error as CommonError;

#[derive(Debug)]
pub enum Error {
    Sys(SysError),
    Biz(BizError),
    Common(CommonError),
}

#[derive(Debug)]
#[repr(i8)]
pub enum BizError {
    // General
    WitnessDataInvalid = 20,

    // Claim
    DistributionDataInvalid,
    MerkleProofInvalid,
}

impl From<SysError> for Error {
    fn from(err: SysError) -> Self {
        Error::Sys(err)
    }
}

impl From<BizError> for Error {
    fn from(err: BizError) -> Self {
        Error::Biz(err)
    }
}

impl From<CommonError> for Error {
    fn from(err: CommonError) -> Self {
        Self::Common(err)
    }
}

impl From<Error> for i8 {
    fn from(err: Error) -> i8 {
        debug!("distribution lock error {:?}", err);
        match err {
            Error::Sys(v) => match v {
                SysError::IndexOutOfBound => 1,
                SysError::ItemMissing => 2,
                SysError::LengthNotEnough(_) => 3,
                SysError::Encoding => 4,
                _ => panic!("unexpected sys error"),
            },
            Error::Biz(v) => v as i8,
            Error::Common(v) => v as i8,
        }
    }
}
