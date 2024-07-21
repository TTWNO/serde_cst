use core::error;
use core::fmt::{self, Display, Formatter};
use core::num::ParseIntError;
use core::result;
use core::str::Utf8Error;

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
    TrailingBytes,
    Message(String),
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
impl serde::de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

pub type Result<T> = result::Result<T, Error>;
