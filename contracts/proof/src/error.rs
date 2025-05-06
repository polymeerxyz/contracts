use ckb_std::error::SysError;
use common::error::Error as CommonError;

#[derive(Debug)]
pub enum Error {
    Sys(SysError),
    Proof(ProofError),
    Common(CommonError),
}

#[derive(Debug)]
#[repr(i8)]
pub enum ProofError {
    OperationNotAllowed = 50,
    VerificationWitnessNotFound,
}

impl From<SysError> for Error {
    fn from(err: SysError) -> Self {
        Error::Sys(err)
    }
}

impl From<CommonError> for Error {
    fn from(err: CommonError) -> Self {
        Error::Common(err)
    }
}

impl From<ProofError> for Error {
    fn from(err: ProofError) -> Self {
        Error::Proof(err)
    }
}

impl From<ProofError> for i8 {
    fn from(err: ProofError) -> Self {
        err as i8
    }
}

impl From<Error> for i8 {
    fn from(err: Error) -> Self {
        match err {
            Error::Sys(sys_err) => match sys_err {
                SysError::IndexOutOfBound => 1,
                SysError::ItemMissing => 2,
                SysError::LengthNotEnough(_) => 3,
                SysError::Encoding => 4,
                _ => panic!("unexpected sys error"),
            },
            Error::Proof(err) => err.into(),
            Error::Common(err) => err.into(),
        }
    }
}
