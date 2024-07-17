use core::error;
use core::num::ParseIntError;
use core::fmt::{Formatter, self, Display};
use core::str::Utf8Error;
use core::result;

#[derive(Debug)]
pub enum Error {
    Eof,
    InvalidHeader,
    ExpectedSize(usize, usize),
    ExpectedBool,
    NotUtf8(Utf8Error),
    ParseInt(ParseIntError),
    WrongLength(usize),
    FieldNotFound(&'static str),
    TrailingBytes
}
impl From<Utf8Error> for Error {
    fn from(utf8e: Utf8Error) -> Error {
        Error::NotUtf8(utf8e)
    }
}
impl From<ParseIntError> for Error {
    fn from(pie: ParseIntError) -> Error {
        Error::ParseInt(pie)
    }
}
impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("Error?")
    }
}
impl error::Error for Error {}
impl serde::de::Error  for Error {
    fn custom<T>(msg: T) -> Self {
        todo!()
    }
}

pub type Result<T> = result::Result<T, Error>;
