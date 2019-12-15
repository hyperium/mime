//! Internal types for the `mime` crate.
//!
//! Nothing to see here. Move along.

use std::error::Error;
use std::{fmt, slice};

pub mod constants;
mod rfc7231;

use self::constants::Atoms;
use self::sealed::Sealed;

pub struct Parser {
    can_range: bool,
}

#[derive(Clone)]
pub struct Mime {
    source: Source,
    slash: u16,
    plus: Option<u16>,
    params: ParamSource,
}

#[derive(Clone)]
pub enum Source {
    Atom(u8, &'static str),
    Dynamic(String),
}

impl AsRef<str> for Source {
    fn as_ref(&self) -> &str {
        match *self {
            Source::Atom(_, s) => s,
            Source::Dynamic(ref s) => s,
        }
    }
}

type Indexed = (u16, u16);
type IndexedPair = (Indexed, Indexed);

#[derive(Clone)]
pub enum ParamSource {
    None,
    Utf8(u16),
    One(u16, IndexedPair),
    Two(u16, IndexedPair, IndexedPair),
    Custom(u16, Vec<IndexedPair>),
}

pub enum InternParams {
    Utf8(usize),
    None,
}

#[derive(Debug)]
pub enum ParseError {
    MissingSlash,
    MissingEqual,
    MissingQuote,
    InvalidToken {
        pos: usize,
        byte: Byte,
    },
    InvalidRange,
    TooLong,
}

#[derive(Clone, Copy)]
pub struct Byte(u8);

impl fmt::Debug for Byte {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            b'\n' => f.write_str("'\\n'"),
            b'\r' => f.write_str("'\\r'"),
            b'\t' => f.write_str("'\\t'"),
            b'\\' => f.write_str("'\\'"),
            b'\0' => f.write_str("'\\0'"),
            0x20..=0x7f => write!(f, "'{}'", self.0 as char),
            _ => write!(f, "'\\x{:02x}'", self.0),
        }
    }
}

impl Error for ParseError {
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = match self {
            ParseError::MissingSlash => "a slash (/) was missing between the type and subtype",
            ParseError::MissingEqual => "an equals sign (=) was missing between a parameter and its value",
            ParseError::MissingQuote => "a quote (\") was missing from a parameter value",
            ParseError::InvalidToken { .. } => "invalid token",
            ParseError::InvalidRange => "unexpected asterisk",
            ParseError::TooLong => "the string is too long",
        };
        if let ParseError::InvalidToken { pos, byte } = *self {
            write!(f, "{}, {:?} at position {}", description, byte, pos)
        } else {
            f.write_str(description)
        }
    }
}

// ===== impl Mime =====

impl Mime {
    #[inline]
    pub fn type_(&self) -> &str {
        &self.source.as_ref()[..self.slash as usize]
    }

    #[inline]
    pub fn subtype(&self) -> &str {
        let end = self.semicolon_or_end();
        &self.source.as_ref()[self.slash as usize + 1..end]
    }

    #[doc(hidden)]
    pub fn private_subtype_offset(&self) -> u16 {
        self.slash
    }

    #[inline]
    pub fn suffix(&self) -> Option<&str> {
        let end = self.semicolon_or_end();
        self.plus.map(|idx| &self.source.as_ref()[idx as usize + 1..end])
    }

    #[doc(hidden)]
    pub fn private_suffix_offset(&self) -> Option<u16> {
        self.plus
    }

    #[inline]
    pub fn params(&self) -> Params {
        let inner = match self.params {
            ParamSource::Utf8(_) => ParamsInner::Utf8,
            ParamSource::One(_, a) => ParamsInner::Inlined(&self.source, Inline::One(a)),
            ParamSource::Two(_, a, b) => ParamsInner::Inlined(&self.source, Inline::Two(a, b)),
            ParamSource::Custom(_, ref params) => {
                ParamsInner::Custom {
                    source: &self.source,
                    params: params.iter(),
                }
            }
            ParamSource::None => ParamsInner::None,
        };

        Params(inner)
    }

    #[doc(hidden)]
    pub fn private_params_source(&self) -> &ParamSource {
        &self.params
    }

    pub fn param<'a>(&'a self, attr: &str) -> Option<&'a str> {
        self.params().find(|e| attr == e.0).map(|e| e.1)
    }

    #[inline]
    pub fn has_params(&self) -> bool {
        self.semicolon().is_some()
    }

    #[inline]
    pub fn without_params(self) -> Self {
        let semicolon = match self.semicolon() {
            None => return self,
            Some(i) => i,
        };

        let mut mtype = self;
        mtype.params = ParamSource::None;
        mtype.source = Atoms::intern(
            &mtype.source.as_ref()[..semicolon],
            mtype.slash,
            InternParams::None,
        );
        mtype
    }

    #[inline]
    fn semicolon(&self) -> Option<usize> {
        match self.params {
            ParamSource::Utf8(i) |
            ParamSource::One(i, ..) |
            ParamSource::Two(i, ..) |
            ParamSource::Custom(i, _) => Some(i as usize),
            ParamSource::None => None,
        }
    }

    #[inline]
    fn semicolon_or_end(&self) -> usize {
        self.semicolon().unwrap_or_else(|| self.source.as_ref().len())
    }

    #[doc(hidden)]
    pub fn private_atom(&self) -> u8 {
        self.atom()
    }

    fn atom(&self) -> u8 {
        match self.source {
            Source::Atom(a, _) => a,
            Source::Dynamic(_) => 0,
        }
    }

    pub fn essence(&self) -> &str {
        &self.source.as_ref()[..self.semicolon_or_end()]
    }

    #[doc(hidden)]
    pub const unsafe fn private_from_proc_macro(
        source: Source,
        slash: u16,
        plus: Option<u16>,
        params: ParamSource,
    ) -> Mime {
        Mime {
            source,
            slash,
            plus,
            params,
        }
    }
}

impl AsRef<str> for Mime {
    #[inline]
    fn as_ref(&self) -> &str {
        self.source.as_ref()
    }
}

impl fmt::Debug for Mime {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self.source.as_ref(), f)
    }
}

impl fmt::Display for Mime {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self.source.as_ref(), f)
    }
}

#[inline]
fn as_u16(i: usize) -> u16 {
    debug_assert!(i <= std::u16::MAX as usize, "as_u16 overflow");
    i as u16
}

#[inline]
fn range(index: (u16, u16)) -> std::ops::Range<usize> {
    index.0 as usize .. index.1 as usize
}

// ===== impl Parser =====

impl Parser {
    #[inline]
    pub fn can_range() -> Self {
        Parser {
            can_range: true,
        }
    }

    #[inline]
    pub fn cannot_range() -> Self {
        Parser {
            can_range: false,
        }
    }

    pub fn parse(&self, src: impl Parse) -> Result<Mime, ParseError> {
        rfc7231::parse(self, src)
    }
}


fn lower_ascii_with_params(s: &str, semi: usize, params: &[IndexedPair]) -> String {
    let mut owned = s.to_owned();
    owned[..semi].make_ascii_lowercase();

    for &(name, value) in params {
        owned[range(name)].make_ascii_lowercase();
        // Since we just converted this part of the string to lowercase,
        // we can skip the `Name == &str` unicase check and do a faster
        // memcmp instead.
        if &owned[range(name)] == "charset" {
            owned[range(value)].make_ascii_lowercase();
        }
    }

    owned
}


// Params ===================


enum ParamsInner<'a> {
    Utf8,
    Inlined(&'a Source, Inline),
    Custom {
        source: &'a Source,
        params: slice::Iter<'a, IndexedPair>,
    },
    None,
}


enum Inline {
    Done,
    One(IndexedPair),
    Two(IndexedPair, IndexedPair),
}

/// An iterator over the parameters of a MIME.
pub struct Params<'a>(ParamsInner<'a>);

impl<'a> fmt::Debug for Params<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Params").finish()
    }
}

impl<'a> Iterator for Params<'a> {
    type Item = (&'a str, &'a str);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.0 {
            ParamsInner::Utf8 => {
                let value = ("charset", "utf-8");
                self.0 = ParamsInner::None;
                Some(value)
            },
            ParamsInner::Inlined(source, ref mut inline) => {
                let next = match *inline {
                    Inline::Done => {
                        None
                    }
                    Inline::One(one) => {
                        *inline = Inline::Done;
                        Some(one)
                    },
                    Inline::Two(one, two) => {
                        *inline = Inline::One(two);
                        Some(one)
                    },
                };
                next.map(|(name, value)| {
                    let name = &source.as_ref()[range(name)];
                    let value = &source.as_ref()[range(value)];
                    (name, value)
                })
            },
            ParamsInner::Custom { source, ref mut params } => {
                params.next().map(|&(name, value)| {
                    let name = &source.as_ref()[range(name)];
                    let value = &source.as_ref()[range(value)];
                    (name, value)
                })
            },
            ParamsInner::None => None,
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.0 {
            ParamsInner::Utf8 => (1, Some(1)),
            ParamsInner::Inlined(_, Inline::Done) => (0, Some(0)),
            ParamsInner::Inlined(_, Inline::One(..)) => (1, Some(1)),
            ParamsInner::Inlined(_, Inline::Two(..)) => (2, Some(2)),
            ParamsInner::Custom { ref params, .. } => params.size_hint(),
            ParamsInner::None => (0, Some(0)),
        }
    }
}

mod sealed {
    pub trait Sealed {
        fn as_str(&self) -> &str;
    }
}

pub trait Parse: Sealed {}

impl<'a> Sealed for &'a str {
    fn as_str(&self) -> &str {
        self
    }
}

impl<'a> Parse for &'a str {}

impl<'a> Sealed for &'a String {
    fn as_str(&self) -> &str {
        *self
    }
}

impl<'a> Parse for &'a String {}

