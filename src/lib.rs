use std::error::Error;
use std::fmt;
use std::fmt::Display;

/// Wraps an error with an explanatory prefix message.
#[derive(Debug)]
pub struct WrappedError<E>
where
    E: Error,
{
    msg: String,
    err: E,
}

impl<E> Error for WrappedError<E> where E: Error {}

impl<E, M> From<(E, M)> for WrappedError<E>
where
    E: Error,
    M: Display,
{
    fn from((err, msg): (E, M)) -> Self {
        let msg = format!("{}", msg);
        WrappedError { msg, err }
    }
}

impl<E> Display for WrappedError<E>
where
    E: Error,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.msg, self.err)
    }
}
