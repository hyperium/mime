use std::error::Error;
use std::fmt;

use mime_parse::ParseError;

/// An invalid `MediaType` or `MediaRange`.
#[derive(Debug)]
pub struct InvalidMime {
    pub(crate) inner: ParseError,
}

impl Error for InvalidMime {
    fn description(&self) -> &str {
        "invalid MIME"
    }
}

impl fmt::Display for InvalidMime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.description(), self.inner)
    }
}
