use std::error::Error;
use std::fmt;

use mime_parse::ParseError;

/// An error type representing an invalid `MediaType` or `MediaRange`.
#[derive(Debug)]
pub struct InvalidMime {
    pub(crate) inner: ParseError,
}

impl Error for InvalidMime {
}

impl fmt::Display for InvalidMime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid MIME: {}", self.inner)
    }
}
