use std::any::type_name;
use std::error::Error as StdError;
use std::fmt::{self, Debug};
use std::marker::PhantomData;

pub enum Error<T> {
    InsertError(String),
    UpdateError(Box<dyn StdError + 'static>),
    Other(Box<dyn StdError + 'static>),
    _Phantom(PhantomData<T>),
}

impl<T> Debug for Error<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl<T> fmt::Display for Error<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error in reference of {}", type_name::<T>())?;

        match self {
            Self::InsertError(msg) => write!(f, "Insert error: {msg}"),
            Self::UpdateError(source) => write!(f, "Update error: {source}"),
            Self::Other(source) => write!(f, "{source}"),
            Self::_Phantom(_) => unreachable!(),
        }
    }
}

impl<T> StdError for Error<T> {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::InsertError(_msg) => None,
            Self::UpdateError(source) => source.source(),
            Self::Other(source) => source.source(),
            Self::_Phantom(_) => unreachable!(),
        }
    }
}
