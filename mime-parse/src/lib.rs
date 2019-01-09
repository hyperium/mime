//! Internal types for the `mime` crate.

use std::collections::HashMap;
use std::error::Error;
use std::{fmt, slice};
use std::iter::Enumerate;
use std::str::Bytes;

pub mod constants;

use self::constants::Atoms;

#[derive(Clone)]
pub struct Mime {
    pub source: Source,
    pub slash: u16,
    pub plus: Option<u16>,
    pub params: ParamSource,
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
        byte: u8,
    },
    InvalidRange,
    TooLong,
}

impl Error for ParseError {
    fn description(&self) -> &str {
        match self {
            ParseError::MissingSlash => "a slash (/) was missing between the type and subtype",
            ParseError::MissingEqual => "an equals sign (=) was missing between a parameter and its value",
            ParseError::MissingQuote => "a quote (\") was missing from a parameter value",
            ParseError::InvalidToken { .. } => "invalid token",
            ParseError::InvalidRange => "unexpected asterisk",
            ParseError::TooLong => "the string is too long",
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let ParseError::InvalidToken { pos, byte } = *self {
            write!(f, "{}, {:X} at position {}", self.description(), byte, pos)
        } else {
            f.write_str(self.description())
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

    #[inline]
    pub fn suffix(&self) -> Option<&str> {
        let end = self.semicolon_or_end();
        self.plus.map(|idx| &self.source.as_ref()[idx as usize + 1..end])
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

    #[inline]
    pub fn has_params(&self) -> bool {
        self.semicolon().is_some()
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

    fn atom(&self) -> u8 {
        match self.source {
            Source::Atom(a, _) => a,
            Source::Dynamic(_) => 0,
        }
    }

    fn eq_type_subtype(&self, other: &Mime) -> bool {
        let left = &self.source.as_ref()[..self.semicolon_or_end()];
        let right = &other.source.as_ref()[..other.semicolon_or_end()];

        left == right
    }

    fn eq_of_params(&self, other: &Mime) -> bool {
        use self::FastEqRes::*;
        // if ParamInner is None or Utf8 we can determine equality faster
        match self.params().fast_eq(&other.params()) {
            Equals => return true,
            NotEquals => return false,
            Undetermined => {},
        }

        // OPTIMIZE: some on-stack structure might be better suited as most
        // media types do not have many parameters
        let my_params = self.params().collect::<HashMap<_,_>>();
        let other_params = self.params().collect::<HashMap<_,_>>();
        my_params == other_params
    }

    pub fn eq_str(&self, s: &str) -> bool {
        if let ParamSource::Utf8(..) = self.params {
            // this only works because ParamSource::Utf8 is only used if
            // its "<type>/<subtype>; charset=utf-8" them moment spaces are
            // set differently or charset is quoted or is utf8 it will not
            // use ParamSource::Utf8
            if self.source.as_ref().len() == s.len() {
                self.source.as_ref().eq_ignore_ascii_case(s)
            } else {
                //OPTIMIZE: once the parser is rewritten and more modular
                // we can use parts of the parser to parse the string without
                // actually crating a mime, and use that for comparision
                //
                parse(s, CanRange::Yes)
                    .map(|other_mime| {
                        self == &other_mime
                    })
                    .unwrap_or(false)
            }
        } else if self.has_params() {
            parse(s, CanRange::Yes)
                .map(|other_mime| {
                    self == &other_mime
                })
                .unwrap_or(false)
        } else {
            self.source.as_ref().eq_ignore_ascii_case(s)
        }
    }
}

impl PartialEq for Mime {
    #[inline]
    fn eq(&self, other: &Mime) -> bool {
        match (self.atom(), other.atom()) {
            // If either atom is 0, it is "dynamic" and needs to be compared
            // slowly...
            (0, _) | (_, 0) => {
                self.eq_type_subtype(other) && self.eq_of_params(other)
            },
            (a, b) => a == b,
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

#[derive(PartialEq)]
pub enum CanRange {
    Yes,
    No,
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

pub fn parse(s: &str, can_range: CanRange) -> Result<Mime, ParseError> {
    if s.len() > std::u16::MAX as usize {
        return Err(ParseError::TooLong);
    }

    if s == "*/*" {
        return match can_range {
            CanRange::Yes => Ok(constants::STAR_STAR),
            CanRange::No => Err(ParseError::InvalidRange),
        };
    }

    let mut iter = s.bytes().enumerate();
    // toplevel
    let mut start;
    let slash;
    loop {
        match iter.next() {
            Some((_, c)) if is_token(c) => (),
            Some((i, b'/')) if i > 0 => {
                slash = as_u16(i);
                start = i + 1;
                break;
            },
            None => return Err(ParseError::MissingSlash), // EOF and no toplevel is no Mime
            Some((pos, byte)) => return Err(ParseError::InvalidToken {
                pos: pos,
                byte: byte,
            }),
        };
    }

    // sublevel
    let mut plus = None;
    loop {
        match iter.next() {
            Some((i, b'+')) if i > start => {
                plus = Some(as_u16(i));
            },
            Some((i, b';')) if i > start => {
                start = i;
                break;
            },

            Some((i, b'*')) if i == start && can_range == CanRange::Yes => {
                // sublevel star can only be the first character, and the next
                // must either be the end, or `;`
                match iter.next() {
                    Some((i, b';')) => {
                        start = i;
                        break;
                    },
                    None => return Ok(Mime {
                        source: Atoms::intern(s, slash, InternParams::None),
                        slash,
                        plus,
                        params: ParamSource::None,
                    }),
                    Some((pos, byte)) => return Err(ParseError::InvalidToken {
                        pos,
                        byte,
                    }),
                }
            },

            Some((_, c)) if is_token(c) => (),
            None => {
                return Ok(Mime {
                    source: Atoms::intern(s, slash, InternParams::None),
                    slash,
                    plus,
                    params: ParamSource::None,
                });
            },
            Some((pos, byte)) => return Err(ParseError::InvalidToken {
                pos: pos,
                byte: byte,
            })
        };
    }

    // params
    let params = params_from_str(s, &mut iter, start)?;

    let source = match params {
        ParamSource::None => Atoms::intern(s, slash, InternParams::None),
        ParamSource::Utf8(semicolon) => Atoms::intern(s, slash, InternParams::Utf8(semicolon as usize)),
        ParamSource::One(semicolon, a) => Source::Dynamic(lower_ascii_with_params(s, semicolon as usize, &[a])),
        ParamSource::Two(semicolon, a, b) => Source::Dynamic(lower_ascii_with_params(s, semicolon as usize, &[a, b])),
        ParamSource::Custom(semicolon, ref indices) => Source::Dynamic(lower_ascii_with_params(s, semicolon as usize, indices)),
    };

    Ok(Mime {
        source,
        slash,
        plus,
        params,
    })
}


fn params_from_str(s: &str, iter: &mut Enumerate<Bytes>, mut start: usize) -> Result<ParamSource, ParseError> {
    let semicolon = as_u16(start);
    start += 1;
    let mut params = ParamSource::None;
    'params: while start < s.len() {
        let name;
        // name
        'name: loop {
            match iter.next() {
                Some((i, b' ')) if i == start => start = i + 1,
                Some((_, c)) if is_token(c) => (),
                Some((i, b'=')) if i > start => {
                    name = (as_u16(start), as_u16(i));
                    start = i + 1;
                    break 'name;
                },
                None => return Err(ParseError::MissingEqual),
                Some((pos, byte)) => return Err(ParseError::InvalidToken {
                    pos: pos,
                    byte: byte,
                }),
            }
        }

        let value;
        // values must be restrict-name-char or "anything goes"
        let mut is_quoted = false;
        let mut is_quoted_pair = false;

        'value: loop {
            if is_quoted {
                if is_quoted_pair {
                    is_quoted_pair = false;
                    match iter.next() {
                        Some((_, ch)) if is_restricted_quoted_char(ch) => (),
                        Some((pos, byte)) => return Err(ParseError::InvalidToken {
                            pos: pos,
                            byte: byte,
                        }),
                        None => return Err(ParseError::MissingQuote),
                    }

                } else {
                    match iter.next() {
                        Some((i, b'"')) if i > start => {
                            value = (as_u16(start), as_u16(i + 1));
                            break 'value;
                        },
                        Some((_, b'\\')) => is_quoted_pair = true,
                        Some((_, c)) if is_restricted_quoted_char(c) => (),
                        None => return Err(ParseError::MissingQuote),
                        Some((pos, byte)) => return Err(ParseError::InvalidToken {
                            pos: pos,
                            byte: byte,
                        }),
                    }
                }
            } else {
                match iter.next() {
                    Some((i, b'"')) if i == start => {
                        is_quoted = true;
                        start = i;
                    },
                    Some((_, c)) if is_token(c) => (),
                    Some((i, b';')) if i > start => {
                        value = (as_u16(start), as_u16(i));
                        start = i + 1;
                        break 'value;
                    }
                    None => {
                        value = (as_u16(start), as_u16(s.len()));
                        start = s.len();
                        break 'value;
                    },

                    Some((pos, byte)) => return Err(ParseError::InvalidToken {
                        pos: pos,
                        byte: byte,
                    }),
                }
            }
        }

        if is_quoted {
            'ws: loop {
                match iter.next() {
                    Some((i, b';')) => {
                        // next param
                        start = i + 1;
                        break 'ws;
                    },
                    Some((_, b' ')) => {
                        // skip whitespace
                    },
                    None => {
                        // eof
                        start = s.len();
                        break 'ws;
                    },
                    Some((pos, byte)) => return Err(ParseError::InvalidToken {
                        pos: pos,
                        byte: byte,
                    }),
                }
            }
        }

        match params {
            ParamSource::Utf8(i) => {
                let i = i + 2;
                let charset = (i, "charset".len() as u16 + i);
                let utf8 = (charset.1 + 1, charset.1 + "utf-8".len() as u16 + 1);
                params = ParamSource::Two(semicolon, (charset, utf8), (name, value));
            },
            ParamSource::One(sc, a) => {
                params = ParamSource::Two(sc, a, (name, value));
            },
            ParamSource::Two(sc, a, b) => {
                params = ParamSource::Custom(sc, vec![a, b, (name, value)]);
            },
            ParamSource::Custom(_, ref mut vec) => {
                vec.push((name, value));
            },
            ParamSource::None => {
                if semicolon + 2 == name.0 && "charset".eq_ignore_ascii_case(&s[name.0 as usize .. name.1 as usize]) &&
                    "utf-8".eq_ignore_ascii_case(&s[value.0 as usize .. value.1 as usize]) {
                    params = ParamSource::Utf8(semicolon);
                    continue 'params;
                }
                params = ParamSource::One(semicolon, (name, value));
            },
        }
    }
    Ok(params)
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

// From [RFC6838](http://tools.ietf.org/html/rfc6838#section-4.2):
//
// > All registered media types MUST be assigned top-level type and
// > subtype names.  The combination of these names serves to uniquely
// > identify the media type, and the subtype name facet (or the absence
// > of one) identifies the registration tree.  Both top-level type and
// > subtype names are case-insensitive.
// >
// > Type and subtype names MUST conform to the following ABNF:
// >
// >     type-name = restricted-name
// >     subtype-name = restricted-name
// >
// >     restricted-name = restricted-name-first *126restricted-name-chars
// >     restricted-name-first  = ALPHA / DIGIT
// >     restricted-name-chars  = ALPHA / DIGIT / "!" / "#" /
// >                              "$" / "&" / "-" / "^" / "_"
// >     restricted-name-chars =/ "." ; Characters before first dot always
// >                                  ; specify a facet name
// >     restricted-name-chars =/ "+" ; Characters after last plus always
// >                                  ; specify a structured syntax suffix

// However, [HTTP](https://tools.ietf.org/html/rfc7231#section-3.1.1.1):
//
// >     media-type = type "/" subtype *( OWS ";" OWS parameter )
// >     type       = token
// >     subtype    = token
// >     parameter  = token "=" ( token / quoted-string )
//
// Where token is defined as:
//
// >     token = 1*tchar
// >     tchar = "!" / "#" / "$" / "%" / "&" / "'" / "*" / "+" / "-" / "." /
// >        "^" / "_" / "`" / "|" / "~" / DIGIT / ALPHA
//
// So, clearly, ¯\_(Ä_/¯

macro_rules! byte_map {
    ($($flag:expr,)*) => ([
        $($flag != 0,)*
    ])
}

static TOKEN_MAP: [bool; 256] = byte_map![
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 1, 0, 1, 1, 1, 1, 1, 0, 0, 0, 1, 0, 1, 1, 0,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0,
    0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

fn is_token(c: u8) -> bool {
    TOKEN_MAP[c as usize]
}

fn is_restricted_quoted_char(c: u8) -> bool {
    c == 9 || (c > 31 && c != 127)
}

#[test]
fn test_lookup_tables() {
    for (i, &valid) in TOKEN_MAP.iter().enumerate() {
        let i = i as u8;
        let should = match i {
            b'a'...b'z' |
            b'A'...b'Z' |
            b'0'...b'9' |
            b'!' |
            b'#' |
            b'$' |
            b'%' |
            b'&' |
            b'\'' |
            b'+' |
            b'-' |
            b'.' |
            b'^' |
            b'_' |
            b'`' |
            b'|' |
            b'~' => true,
            _ => false
        };
        assert_eq!(valid, should, "{:?} ({}) should be {}", i as char, i, should);
    }
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

enum FastEqRes {
    Equals,
    NotEquals,
    Undetermined
}

/// An iterator over the parameters of a MIME.
pub struct Params<'a>(ParamsInner<'a>);

impl<'a> fmt::Debug for Params<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Params").finish()
    }
}

impl<'a> Params<'a> {
    fn fast_eq<'b>(&self, other: &Params<'b>) -> FastEqRes {
        match (&self.0, &other.0) {
            (&ParamsInner::None, &ParamsInner::None) |
            (&ParamsInner::Utf8, &ParamsInner::Utf8) => FastEqRes::Equals,

            (&ParamsInner::None, _) |
            (_, &ParamsInner::None)  => FastEqRes::NotEquals,

            _ => FastEqRes::Undetermined,
        }
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

