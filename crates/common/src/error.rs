use ckb_std::error::SysError;

#[derive(Debug)]
#[repr(i8)]
pub enum Error {
    IndexOutOfBound = 1,
    ItemMissing,
    LengthNotEnough,
    Encoding,

    InvalidArgumentCount = 10,
    InvalidArgumentLength,
    InvalidScriptHash,
    MissingScriptHash,
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
