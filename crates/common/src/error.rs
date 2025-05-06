use ckb_std::error::SysError;

#[derive(Debug)]
pub enum Error {
    Sys(SysError),
    Validation(ValidationError),
    TypeID(TypeIDError),
}

#[derive(Debug)]
#[repr(i8)]
pub enum ValidationError {
    // Campaign ID not match
    CampaignIDNotMatch = 10,
    // Entity ID not match
    EntityIDNotMatch,
    // Merkle root not match
    ProofNotMatch,
    // Checkpoint messages not found
    CheckpointsNotFound,
}

#[derive(Debug)]
#[repr(i8)]
pub enum TypeIDError {
    // There can only be at most one input and at most one output type ID cell
    InvalidTypeIDCellNum = 20,
    // Type id does not match args
    TypeIDNotMatch,
    // Length of type id is incorrect
    ArgsLengthNotEnough,
    // Type id not found
    TypeIDNotFound,
}

impl From<SysError> for Error {
    fn from(err: SysError) -> Self {
        Error::Sys(err)
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
            Error::Validation(err) => err as i8,
            Error::TypeID(err) => err as i8,
        }
    }
}
