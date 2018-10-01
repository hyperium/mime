#![doc(html_root_url = "https://docs.rs/mime/0.3.6")]
#![deny(warnings)]
#![deny(missing_docs)]
#![deny(missing_debug_implementations)]

//! # MediaType and MediaRange
//!
//! ## What is MediaType?
//!
//! Example mime string: `text/plain`
//!
//! ```
//! let plain_text: mime::MediaType = "text/plain".parse().unwrap();
//! assert_eq!(plain_text, mime::TEXT_PLAIN);
//! ```
//!
//! ## Inspecting Mimes
//!
//! ```
//! let mime = mime::TEXT_PLAIN;
//! match (mime.type_(), mime.subtype()) {
//!     (mime::TEXT, mime::PLAIN) => println!("plain text!"),
//!     (mime::TEXT, _) => println!("structured text"),
//!     _ => println!("not text"),
//! }
//! ```


extern crate quoted_string;

use std::cmp::Ordering;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use std::slice;

pub use self::name::Name;
pub use self::value::Value;

mod name;
mod parse;
mod value;

/// A parsed MIME or media type.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MediaType {
    mime: Mime,
}

/// A parsed media range used to match media types.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct MediaRange {
    mime: Mime,
}

#[derive(Clone)]
struct Mime {
    source: Source,
    slash: usize,
    plus: Option<usize>,
    params: ParamSource,
}

/// An invalid `MediaType` or `MediaRange`.
#[derive(Debug)]
pub struct InvalidMime {
    inner: parse::ParseError,
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

#[derive(Clone)]
enum Source {
    Atom(u8, &'static str),
    Dynamic(String),
}

impl Source {
    fn as_ref(&self) -> &str {
        match *self {
            Source::Atom(_, s) => s,
            Source::Dynamic(ref s) => s,
        }
    }
}

#[derive(Clone)]
enum ParamSource {
    Utf8(usize),
    Custom(usize, Vec<(Indexed, Indexed)>),
    None,
}

#[derive(Clone, Copy)]
struct Indexed(usize, usize);

// ==== impl MediaType =====

impl MediaType {
    /// Get the top level media type for this `MediaType`.
    ///
    /// # Example
    ///
    /// ```
    /// let mime = mime::TEXT_PLAIN;
    /// assert_eq!(mime.type_(), "text");
    /// assert_eq!(mime.type_(), mime::TEXT);
    /// ```
    #[inline]
    pub fn type_(&self) -> Name {
        self.mime.type_()
    }

    /// Get the subtype of this `MediaType`.
    ///
    /// # Example
    ///
    /// ```
    /// let mime = mime::TEXT_PLAIN;
    /// assert_eq!(mime.subtype(), "plain");
    /// assert_eq!(mime.subtype(), mime::PLAIN);
    /// ```
    #[inline]
    pub fn subtype(&self) -> Name {
        self.mime.subtype()
    }

    /// Get an optional +suffix for this `MediaType`.
    ///
    /// # Example
    ///
    /// ```
    /// let svg = "image/svg+xml".parse::<mime::MediaType>().unwrap();
    /// assert_eq!(svg.suffix(), Some(mime::XML));
    /// assert_eq!(svg.suffix().unwrap(), "xml");
    ///
    ///
    /// assert!(mime::TEXT_PLAIN.suffix().is_none());
    /// ```
    #[inline]
    pub fn suffix(&self) -> Option<Name> {
        self.mime.suffix()
    }

    /// Look up a parameter by name.
    ///
    /// # Example
    ///
    /// ```
    /// let mime = mime::TEXT_PLAIN_UTF_8;
    /// assert_eq!(mime.get_param(mime::CHARSET), Some(mime::UTF_8));
    /// assert_eq!(mime.get_param("charset").unwrap(), "utf-8");
    /// assert!(mime.get_param("boundary").is_none());
    ///
    /// let mime = "multipart/form-data; boundary=ABCDEFG".parse::<mime::MediaType>().unwrap();
    /// assert_eq!(mime.get_param(mime::BOUNDARY).unwrap(), "ABCDEFG");
    /// ```
    pub fn get_param<'a, N>(&'a self, attr: N) -> Option<Value<'a>>
    where
        N: PartialEq<Name<'a>>,
    {
        self.mime.get_param(attr)
    }

    /// Returns an iterator over the parameters.
    ///
    /// # Example
    ///
    /// ```
    /// let pkcs7: mime::MediaType =
    ///     "application/pkcs7-mime; smime-type=enveloped-data; name=smime.p7m".parse().unwrap();
    ///
    /// let (names, values): (Vec<_>, Vec<_>) = pkcs7.params().unzip();
    ///
    /// assert_eq!(names, &["smime-type", "name"]);
    /// assert_eq!(values, &["enveloped-data", "smime.p7m"]);
    /// ```
    #[inline]
    pub fn params(&self) -> Params {
        self.mime.params()
    }

    /// Returns true if the media type has at last one parameter.
    ///
    /// # Example
    ///
    /// ```
    /// let plain_text: mime::MediaType = "text/plain".parse().unwrap();
    /// assert_eq!(plain_text.has_params(), false);
    ///
    /// let plain_text_utf8: mime::MediaType = "text/plain; charset=utf-8".parse().unwrap();
    /// assert_eq!(plain_text_utf8.has_params(), true);
    /// ```
    #[inline]
    pub fn has_params(&self) -> bool {
        self.mime.has_params()
    }

    #[cfg(test)]
    fn test_assert_asterisks(&self) {
        assert!(!self.as_ref().contains('*'), "{:?} contains an asterisk", self);
    }
}

impl PartialEq<str> for MediaType {
    fn eq(&self, s: &str) -> bool {
        self.mime == s
    }
}

impl<'a> PartialEq<&'a str> for MediaType {
    #[inline]
    fn eq(&self, s: & &'a str) -> bool {
        self.mime == *s
    }
}

impl<'a> PartialEq<MediaType> for &'a str {
    #[inline]
    fn eq(&self, mt: &MediaType) -> bool {
        mt.mime == *self
    }
}

impl PartialEq<MediaType> for str {
    #[inline]
    fn eq(&self, mt: &MediaType) -> bool {
        mt.mime == self
    }
}

impl FromStr for MediaType {
    type Err = InvalidMime;

    fn from_str(s: &str) -> Result<MediaType, Self::Err> {
        parse::parse(s, parse::CanRange::No)
            .map(|mime| MediaType { mime })
            .map_err(|e| InvalidMime { inner: e })
    }
}

impl AsRef<str> for MediaType {
    fn as_ref(&self) -> &str {
        self.mime.as_ref()
    }
}

impl fmt::Debug for MediaType {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.mime, f)
    }
}

impl fmt::Display for MediaType {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.mime, f)
    }
}

// ===== impl MediaRange =====

impl MediaRange {

    /// Get the top level media type for this `MediaRange`.
    ///
    /// # Example
    ///
    /// ```
    /// let exact = mime::MediaRange::from(mime::TEXT_PLAIN);
    /// assert_eq!(exact.type_(), "text");
    /// assert_eq!(exact.type_(), mime::TEXT);
    /// ```
    #[inline]
    pub fn type_(&self) -> Name {
        self.mime.type_()
    }

    /// Get the subtype of this `MediaRange`.
    ///
    /// # Example
    ///
    /// ```
    /// let range = "text/*; charset=utf-8"
    ///     .parse::<mime::MediaRange>()
    ///     .unwrap();
    ///
    /// assert_eq!(range.subtype(), "*");
    /// assert_eq!(range.subtype(), mime::STAR);
    ///
    /// let exact = mime::MediaRange::from(mime::TEXT_PLAIN);
    /// assert_eq!(exact.subtype(), mime::PLAIN);
    /// assert_eq!(exact.subtype(), "plain");
    /// ```
    #[inline]
    pub fn subtype(&self) -> Name {
        self.mime.subtype()
    }

    /// Get an optional +suffix for this `MediaRange`.
    ///
    /// # Example
    ///
    /// ```
    /// let svg = "image/svg+xml"
    ///     .parse::<mime::MediaRange>()
    ///     .unwrap();
    ///
    /// assert_eq!(svg.suffix(), Some(mime::XML));
    /// assert_eq!(svg.suffix().unwrap(), "xml");
    ///
    ///
    /// let any = "*/*"
    ///     .parse::<mime::MediaRange>()
    ///     .unwrap();
    ///
    /// assert_eq!(any.suffix(), None);
    /// ```
    #[inline]
    pub fn suffix(&self) -> Option<Name> {
        self.mime.suffix()
    }

    /// Checks if this `MediaRange` matches a specific `MediaType`.
    ///
    /// # Example
    ///
    /// ```
    /// let images = "image/*"
    ///     .parse::<mime::MediaRange>()
    ///     .unwrap();
    ///
    /// assert!(images.matches(&mime::IMAGE_JPEG));
    /// assert!(images.matches(&mime::IMAGE_PNG));
    ///
    /// assert!(!images.matches(&mime::TEXT_PLAIN));
    /// ```
    pub fn matches(&self, mt: &MediaType) -> bool {
        let type_ = self.type_();

        if type_ == STAR {
            return true;
        }

        if type_ != mt.type_() {
            return false;
        }

        let subtype = self.subtype();

        if subtype == STAR {
            return true;
        }

        if subtype != mt.subtype() {
            return false;
        }

        for (name, value) in self.params() {
            if mt.get_param(name) != Some(value) {
                return false;
            }
        }

        true
    }

    /// Look up a parameter by name.
    ///
    /// # Example
    ///
    /// ```
    /// let range = "text/*; charset=utf-8"
    ///     .parse::<mime::MediaRange>()
    ///     .unwrap();
    ///
    /// assert_eq!(range.get_param(mime::CHARSET), Some(mime::UTF_8));
    /// assert_eq!(range.get_param("charset").unwrap(), "utf-8");
    /// assert_eq!(range.get_param("boundary"), None);
    /// ```
    pub fn get_param<'a, N>(&'a self, attr: N) -> Option<Value<'a>>
    where
        N: PartialEq<Name<'a>>,
    {
        self.mime.get_param(attr)
    }

    /// Returns an iterator over the parameters.
    ///
    /// # Example
    ///
    /// ```
    /// let pkcs7: mime::MediaRange =
    ///     "application/pkcs7-mime; smime-type=enveloped-data; name=smime.p7m".parse().unwrap();
    ///
    /// let (names, values): (Vec<_>, Vec<_>) = pkcs7.params().unzip();
    ///
    /// assert_eq!(names, &["smime-type", "name"]);
    /// assert_eq!(values, &["enveloped-data", "smime.p7m"]);
    /// ```
    #[inline]
    pub fn params(&self) -> Params {
        self.mime.params()
    }

    /// Returns true if the media type has at last one parameter.
    ///
    /// # Example
    ///
    /// ```
    /// let plain_text: mime::MediaType = "text/plain".parse().unwrap();
    /// assert_eq!(plain_text.has_params(), false);
    ///
    /// let plain_text_utf8: mime::MediaType = "text/plain; charset=utf-8".parse().unwrap();
    /// assert_eq!(plain_text_utf8.has_params(), true);
    /// ```
    #[inline]
    pub fn has_params(&self) -> bool {
        self.mime.has_params()
    }

    #[cfg(test)]
    fn test_assert_asterisks(&self) {
        // asterisks are allowed in MediaRange constants
    }
}

impl From<MediaType> for MediaRange {
    fn from(mt: MediaType) -> MediaRange {
        MediaRange {
            mime: mt.mime,
        }
    }
}

impl PartialEq<str> for MediaRange {
    fn eq(&self, s: &str) -> bool {
        self.mime == s
    }
}

impl<'a> PartialEq<&'a str> for MediaRange {
    #[inline]
    fn eq(&self, s: & &'a str) -> bool {
        self.mime == *s
    }
}

impl<'a> PartialEq<MediaRange> for &'a str {
    #[inline]
    fn eq(&self, mt: &MediaRange) -> bool {
        mt.mime == *self
    }
}

impl PartialEq<MediaRange> for str {
    #[inline]
    fn eq(&self, mt: &MediaRange) -> bool {
        mt.mime == self
    }
}

impl FromStr for MediaRange {
    type Err = InvalidMime;

    fn from_str(s: &str) -> Result<MediaRange, Self::Err> {
        parse::parse(s, parse::CanRange::Yes)
            .map(|mime| MediaRange { mime })
            .map_err(|e| InvalidMime { inner: e })
    }
}

impl AsRef<str> for MediaRange {
    fn as_ref(&self) -> &str {
        self.mime.as_ref()
    }
}

impl fmt::Debug for MediaRange {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.mime, f)
    }
}

impl fmt::Display for MediaRange {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.mime, f)
    }
}

// ===== impl Mime =====

impl Mime {
    fn type_(&self) -> Name {
        Name {
            source: &self.source.as_ref()[..self.slash],
        }
    }

    fn subtype(&self) -> Name {
        let end = self.plus.unwrap_or_else(|| {
            self.semicolon().unwrap_or_else(|| self.source.as_ref().len())
        });
        Name {
            source: &self.source.as_ref()[self.slash + 1..end],
        }
    }

    fn suffix(&self) -> Option<Name> {
        let end = self.semicolon().unwrap_or_else(|| self.source.as_ref().len());
        self.plus.map(|idx| Name {
            source: &self.source.as_ref()[idx + 1..end],
        })
    }

    fn get_param<'a, N>(&'a self, attr: N) -> Option<Value<'a>>
    where
        N: PartialEq<Name<'a>>,
    {
        self.params().find(|e| attr == e.0).map(|e| e.1)
    }

    fn params(&self) -> Params {
        let inner = match self.params {
            ParamSource::Utf8(_) => ParamsInner::Utf8,
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

    fn has_params(&self) -> bool {
        self.semicolon().is_some()
    }

    #[inline]
    fn semicolon(&self) -> Option<usize> {
        match self.params {
            ParamSource::Utf8(i) |
            ParamSource::Custom(i, _) => Some(i),
            ParamSource::None => None,
        }
    }

    fn atom(&self) -> u8 {
        match self.source {
            Source::Atom(a, _) => a,
            _ => 0,
        }
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
}


impl PartialEq for Mime {
    #[inline]
    fn eq(&self, other: &Mime) -> bool {
        match (self.atom(), other.atom()) {
            // TODO:
            // This could optimize for when there are no customs parameters.
            // Any parsed mime has already been lowercased, so if there aren't
            // any parameters that are case sensistive, this can skip the
            // unicase::eq_ascii, and just use a memcmp instead.
            (0, _) |
            (_, 0) => {
                self.type_() == other.type_()  &&
                    self.subtype() == other.subtype() &&
                    self.suffix() == other.suffix() &&
                    self.eq_of_params(other)
            },
            (a, b) => a == b,
        }
    }
}

impl Eq for Mime {}

impl PartialOrd for Mime {
    fn partial_cmp(&self, other: &Mime) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Mime {
    fn cmp(&self, other: &Mime) -> Ordering {
        self.source.as_ref().cmp(other.source.as_ref())
    }
}

impl Hash for Mime {
    fn hash<T: Hasher>(&self, hasher: &mut T) {
        hasher.write(self.source.as_ref().as_bytes());
    }
}

impl PartialEq<str> for Mime {
    fn eq(&self, s: &str) -> bool {
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
                parse::parse(s, parse::CanRange::Yes)
                    .map(|other_mime| {
                        self == &other_mime
                    })
                    .unwrap_or(false)
            }
        } else if self.has_params() {
            parse::parse(s, parse::CanRange::Yes)
                .map(|other_mime| {
                    self == &other_mime
                })
                .unwrap_or(false)
        } else {
            self.source.as_ref().eq_ignore_ascii_case(s)
        }
    }
}

impl<'a> PartialEq<&'a str> for Mime {
    #[inline]
    fn eq(&self, s: & &'a str) -> bool {
        self == *s
    }
}

impl<'a> PartialEq<Mime> for &'a str {
    #[inline]
    fn eq(&self, mime: &Mime) -> bool {
        mime == self
    }
}

impl PartialEq<Mime> for str {
    #[inline]
    fn eq(&self, mime: &Mime) -> bool {
        mime == self
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

// Params ===================

enum ParamsInner<'a> {
    Utf8,
    Custom {
        source: &'a Source,
        params: slice::Iter<'a, (Indexed, Indexed)>,
    },
    None,
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
    type Item = (Name<'a>, Value<'a>);

    #[inline]
    fn next(&mut self) -> Option<(Name<'a>, Value<'a>)> {
        match self.0 {
            ParamsInner::Utf8 => {
                let value = (CHARSET, UTF_8);
                self.0 = ParamsInner::None;
                Some(value)
            }
            ParamsInner::Custom { source, ref mut params } => {
                params.next().map(|&(name, value)| {
                    let name = Name {
                        source: &source.as_ref()[name.0..name.1],
                    };
                    let value = Value {
                        source: &source.as_ref()[value.0..value.1],
                        ascii_case_insensitive: name == CHARSET,
                    };
                    (name, value)
                })
            }
            ParamsInner::None => None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.0 {
            ParamsInner::Utf8 => (1, Some(1)),
            ParamsInner::Custom { ref params, .. } => params.size_hint(),
            ParamsInner::None => (0, Some(0)),
        }
    }
}

macro_rules! names {
    ($($id:ident, $e:expr;)*) => (
        $(
        #[doc = $e]
        pub const $id: Name<'static> = Name {
            source: $e,
        };
        )*

        #[test]
        fn test_names_macro_consts() {
            #[allow(deprecated,unused_imports)]
            use std::ascii::AsciiExt;
            $(
            assert_eq!($id.source.to_ascii_lowercase(), $id.source);
            )*
        }
    )
}

names! {
    STAR, "*";

    TEXT, "text";
    IMAGE, "image";
    AUDIO, "audio";
    VIDEO, "video";
    APPLICATION, "application";
    MULTIPART, "multipart";
    MESSAGE, "message";
    MODEL, "model";
    FONT, "font";

    // common text/ *
    PLAIN, "plain";
    HTML, "html";
    XML, "xml";
    JAVASCRIPT, "javascript";
    CSS, "css";
    CSV, "csv";
    EVENT_STREAM, "event-stream";
    VCARD, "vcard";

    // common application/*
    JSON, "json";
    WWW_FORM_URLENCODED, "x-www-form-urlencoded";
    MSGPACK, "msgpack";
    OCTET_STREAM, "octet-stream";
    PDF, "pdf";

    // common font/*
    WOFF, "woff";
    WOFF2, "woff2";

    // multipart/*
    FORM_DATA, "form-data";

    // common image/*
    BMP, "bmp";
    GIF, "gif";
    JPEG, "jpeg";
    PNG, "png";
    SVG, "svg";

    // audio/*
    BASIC, "basic";
    MPEG, "mpeg";
    MP4, "mp4";
    OGG, "ogg";

    // parameters
    CHARSET, "charset";
    BOUNDARY, "boundary";
}

/// a `Value` usable for a charset parameter.
///
/// # Example
/// ```
/// # use mime::{MediaType, CHARSET, UTF_8};
/// let mime = "text/plain; charset=utf-8".parse::<MediaType>().unwrap();
/// assert_eq!(mime.get_param(CHARSET), Some(UTF_8));
/// ```
pub static UTF_8: Value = Value { source: "utf-8", ascii_case_insensitive: true };

macro_rules! mimes {
    ($(@ $kind:ident: $($id:ident, $($piece:expr),+;)+)+) => (
        #[allow(non_camel_case_types)]
        enum __Atoms {
            __Dynamic,
        $($(
            $id,
        )+)+
        }

        const MIME_STAR_STAR: Mime = STAR_STAR.mime;

        $($(
            mime_constant! {
                $kind, $id, $($piece),+
            }
        )+)+

        #[test]
        fn test_mimes_macro_consts() {
            let _ = [
            $($(
            mime_constant_test! {
                $id, $($piece),*
            }
            ,)+)+
            ].iter().enumerate().map(|(pos, &atom)| {
                // + 1 (__Dynamic)
                assert_eq!(pos + 1, atom as usize, "atom {} in position {}", atom, pos + 1);
            }).collect::<Vec<()>>();
        }
    )
}

macro_rules! mime_constant {
    ($kind:ident, $id:ident, $src:expr, $slash:expr) => (
        mime_constant!($kind, $id, $src, $slash, None);
    );
    ($kind:ident, $id:ident, $src:expr, $slash:expr, $plus:expr) => (
        mime_constant!(FULL $kind, $id, $src, $slash, $plus, ParamSource::None);
    );

    ($kind:ident, $id:ident, $src:expr, $slash:expr, $plus:expr, $params:expr) => (
        mime_constant!(FULL $kind, $id, $src, $slash, $plus, ParamSource::Utf8($params));
    );


    (FULL $kind:ident, $id:ident, $src:expr, $slash:expr, $plus:expr, $params:expr) => (
        #[doc = "`"]
        #[doc = $src]
        #[doc = "`"]
        pub const $id: $kind = $kind {
            mime: Mime {
                source: Source::Atom(__Atoms::$id as u8, $src),
                slash: $slash,
                plus: $plus,
                params: $params,
            },
        };
    )
}


#[cfg(test)]
macro_rules! mime_constant_test {
    ($id:ident, $src:expr, $slash:expr) => (
        mime_constant_test!($id, $src, $slash, None);
    );
    ($id:ident, $src:expr, $slash:expr, $plus:expr) => (
        mime_constant_test!(FULL $id, $src, $slash, $plus, ParamSource::None);
    );

    ($id:ident, $src:expr, $slash:expr, $plus:expr, $params:expr) => (
        mime_constant_test!(FULL $id, $src, $slash, $plus, ParamSource::Utf8($params));
    );

    (FULL $id:ident, $src:expr, $slash:expr, $plus:expr, $params:expr) => ({
        let __mime = $id;
        let __slash = __mime.as_ref().as_bytes()[$slash];
        assert_eq!(__slash, b'/', "{:?} has {:?} at slash position {:?}", __mime, __slash as char, $slash);
        if let Some(plus) = __mime.mime.plus {
            let __c = __mime.as_ref().as_bytes()[plus];
            assert_eq!(__c, b'+', "{:?} has {:?} at plus position {:?}", __mime, __c as char, plus);
        } else {
            assert!(!__mime.as_ref().as_bytes().contains(&b'+'), "{:?} forgot plus", __mime);
        }
        if let ParamSource::Utf8(semicolon) = __mime.mime.params {
            assert_eq!(__mime.as_ref().as_bytes()[semicolon], b';');
            assert_eq!(&__mime.as_ref()[semicolon..], "; charset=utf-8");
        } else if let ParamSource::None = __mime.mime.params {
            assert!(!__mime.as_ref().as_bytes().contains(&b';'));
        } else {
            unreachable!();
        }

        // prevent ranges from being MediaTypes
        __mime.test_assert_asterisks();
        __mime.mime.atom()
    })
}


mimes! {
    @ MediaType:

    TEXT_PLAIN, "text/plain", 4;
    TEXT_PLAIN_UTF_8, "text/plain; charset=utf-8", 4, None, 10;
    TEXT_HTML, "text/html", 4;
    TEXT_HTML_UTF_8, "text/html; charset=utf-8", 4, None, 9;
    TEXT_CSS, "text/css", 4;
    TEXT_CSS_UTF_8, "text/css; charset=utf-8", 4, None, 8;
    TEXT_JAVASCRIPT, "text/javascript", 4;
    TEXT_XML, "text/xml", 4;
    TEXT_EVENT_STREAM, "text/event-stream", 4;
    TEXT_CSV, "text/csv", 4;
    TEXT_CSV_UTF_8, "text/csv; charset=utf-8", 4, None, 8;
    TEXT_TAB_SEPARATED_VALUES, "text/tab-separated-values", 4;
    TEXT_TAB_SEPARATED_VALUES_UTF_8, "text/tab-separated-values; charset=utf-8", 4, None, 25;
    TEXT_VCARD, "text/vcard", 4;

    //IMAGE_STAR, "image/*", 5;
    IMAGE_JPEG, "image/jpeg", 5;
    IMAGE_GIF, "image/gif", 5;
    IMAGE_PNG, "image/png", 5;
    IMAGE_BMP, "image/bmp", 5;
    IMAGE_SVG, "image/svg+xml", 5, Some(9);

    FONT_WOFF, "font/woff", 4;
    FONT_WOFF2, "font/woff2", 4;

    APPLICATION_JSON, "application/json", 11;
    APPLICATION_JAVASCRIPT, "application/javascript", 11;
    APPLICATION_JAVASCRIPT_UTF_8, "application/javascript; charset=utf-8", 11, None, 22;
    APPLICATION_WWW_FORM_URLENCODED, "application/x-www-form-urlencoded", 11;
    APPLICATION_OCTET_STREAM, "application/octet-stream", 11;
    APPLICATION_MSGPACK, "application/msgpack", 11;
    APPLICATION_PDF, "application/pdf", 11;
    APPLICATION_DNS, "application/dns-message", 11;

    MULTIPART_FORM_DATA, "multipart/form-data", 9;

    // media-ranges
    @ MediaRange:
    STAR_STAR, "*/*", 1;
    TEXT_STAR, "text/*", 4;
    IMAGE_STAR, "image/*", 5;
    VIDEO_STAR, "video/*", 5;
    AUDIO_STAR, "audio/*", 5;
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use super::*;

    #[test]
    fn test_type_() {
        assert_eq!(TEXT_PLAIN.type_(), TEXT);
    }


    #[test]
    fn test_subtype() {
        assert_eq!(TEXT_PLAIN.subtype(), PLAIN);
        assert_eq!(TEXT_PLAIN_UTF_8.subtype(), PLAIN);
        let mime = MediaType::from_str("text/html+xml").unwrap();
        assert_eq!(mime.subtype(), HTML);
    }

    #[test]
    fn test_matching() {
        match (TEXT_PLAIN.type_(), TEXT_PLAIN.subtype()) {
            (TEXT, PLAIN) => (),
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_suffix() {
        assert_eq!(TEXT_PLAIN.suffix(), None);
        let mime = MediaType::from_str("text/html+xml").unwrap();
        assert_eq!(mime.suffix(), Some(XML));
    }

    #[test]
    fn test_mime_fmt() {
        let mime = TEXT_PLAIN;
        assert_eq!(mime.to_string(), "text/plain");
        let mime = TEXT_PLAIN_UTF_8;
        assert_eq!(mime.to_string(), "text/plain; charset=utf-8");
    }

    #[test]
    fn test_media_type_from_str() {
        assert_eq!(MediaType::from_str("text/plain").unwrap(), TEXT_PLAIN);
        assert_eq!(MediaType::from_str("TEXT/PLAIN").unwrap(), TEXT_PLAIN);
        assert_eq!(MediaType::from_str("text/plain; charset=utf-8").unwrap(), TEXT_PLAIN_UTF_8);
        assert_eq!(MediaType::from_str("text/plain;charset=\"utf-8\"").unwrap(), TEXT_PLAIN_UTF_8);

        // quotes + semi colon
        MediaType::from_str("text/plain;charset=\"utf-8\"; foo=bar").unwrap();
        MediaType::from_str("text/plain;charset=\"utf-8\" ; foo=bar").unwrap();

        let upper = MediaType::from_str("TEXT/PLAIN").unwrap();
        assert_eq!(upper, TEXT_PLAIN);
        assert_eq!(upper.type_(), TEXT);
        assert_eq!(upper.subtype(), PLAIN);


        let extended = MediaType::from_str("TEXT/PLAIN; CHARSET=UTF-8; FOO=BAR").unwrap();
        assert_eq!(extended, "text/plain; charset=utf-8; foo=BAR");
        assert_eq!(extended.get_param("charset").unwrap(), "utf-8");
        assert_eq!(extended.get_param("foo").unwrap(), "BAR");

        MediaType::from_str("multipart/form-data; boundary=--------foobar").unwrap();

        // parse errors
        MediaType::from_str("f o o / bar").unwrap_err();
        MediaType::from_str("text\n/plain").unwrap_err();
        MediaType::from_str("text\r/plain").unwrap_err();
        MediaType::from_str("text/\r\nplain").unwrap_err();
        MediaType::from_str("text/plain;\r\ncharset=utf-8").unwrap_err();
        MediaType::from_str("text/plain; charset=\r\nutf-8").unwrap_err();
        MediaType::from_str("text/plain; charset=\"\r\nutf-8\"").unwrap_err();
    }

    #[test]
    fn media_range_from_str() {
        // exact types
        assert_eq!(MediaRange::from_str("text/plain").unwrap(), MediaRange::from(TEXT_PLAIN));

        // stars
        assert_eq!("*/*".parse::<MediaRange>().unwrap(), "*/*"); //TODO: STAR_STAR);
        assert_eq!("image/*".parse::<MediaRange>().unwrap(), "image/*");
        assert_eq!("text/*; charset=utf-8".parse::<MediaRange>().unwrap(), "text/*; charset=utf-8");

        // bad stars
        MediaRange::from_str("text/*plain").unwrap_err();
    }

    #[test]
    fn test_case_sensitive_values() {
        let mime = MediaType::from_str("multipart/form-data; charset=BASE64; boundary=ABCDEFG").unwrap();
        assert_eq!(mime.get_param(CHARSET).unwrap(), "bAsE64");
        assert_eq!(mime.get_param(BOUNDARY).unwrap(), "ABCDEFG");
        assert_ne!(mime.get_param(BOUNDARY).unwrap(), "abcdefg");
    }

    #[test]
    fn test_get_param() {
        assert_eq!(TEXT_PLAIN.get_param("charset"), None);
        assert_eq!(TEXT_PLAIN.get_param("baz"), None);

        assert_eq!(TEXT_PLAIN_UTF_8.get_param("charset"), Some(UTF_8));
        assert_eq!(TEXT_PLAIN_UTF_8.get_param("baz"), None);

        let mime = MediaType::from_str("text/plain; charset=utf-8; foo=bar").unwrap();
        assert_eq!(mime.get_param(CHARSET).unwrap(), "utf-8");
        assert_eq!(mime.get_param("foo").unwrap(), "bar");
        assert_eq!(mime.get_param("baz"), None);


        let mime = MediaType::from_str("text/plain;charset=\"utf-8\"").unwrap();
        assert_eq!(mime.get_param(CHARSET), Some(UTF_8));
    }

    #[test]
    fn test_mime_with_dquote_quoted_pair() {
        let mime = MediaType::from_str(r#"application/x-custom; title="the \" char""#).unwrap();
        assert_eq!(mime.get_param("title").unwrap(), "the \" char");
    }

    #[test]
    fn test_params() {
        let mime = TEXT_PLAIN;
        let mut params = mime.params();
        assert_eq!(params.next(), None);

        let mime = MediaType::from_str("text/plain; charset=utf-8; foo=bar").unwrap();
        let mut params = mime.params();
        assert_eq!(params.next(), Some((CHARSET, UTF_8)));

        let (second_param_left, second_param_right) = params.next().unwrap();
        assert_eq!(second_param_left, "foo");
        assert_eq!(second_param_right, "bar");

        assert_eq!(params.next(), None);
    }

    #[test]
    fn test_has_params() {
        let mime = TEXT_PLAIN;
        assert_eq!(mime.has_params(), false);

        let mime = MediaType::from_str("text/plain; charset=utf-8").unwrap();
        assert_eq!(mime.has_params(), true);

        let mime = MediaType::from_str("text/plain; charset=utf-8; foo=bar").unwrap();
        assert_eq!(mime.has_params(), true);
    }

    #[test]
    fn test_name_eq() {
        assert_eq!(TEXT, TEXT);
        assert_eq!(TEXT, "text");
        assert_eq!("text", TEXT);
        assert_eq!(TEXT, "TEXT");
    }

    #[test]
    fn test_value_eq() {
        let param = Value {
            source: "ABC",
            ascii_case_insensitive: false,
        };

        assert_eq!(param, param);
        assert_eq!(param, "ABC");
        assert_eq!("ABC", param);
        assert_ne!(param, "abc");
        assert_ne!("abc", param);
    }

    #[test]
    fn test_mime_with_utf8_values() {
        let mime = MediaType::from_str(r#"application/x-custom; param="Straße""#).unwrap();
        assert_eq!(mime.get_param("param").unwrap(), "Straße");
    }

    #[test]
    fn test_mime_with_multiple_plus() {
        let mime = MediaType::from_str(r#"application/x-custom+bad+suffix"#).unwrap();
        assert_eq!(mime.type_(), "application");
        assert_eq!(mime.subtype(), "x-custom+bad");
        assert_eq!(mime.suffix().unwrap(), "suffix");
    }

    #[test]
    fn test_mime_param_with_empty_quoted_string() {
        let mime = MediaType::from_str(r#"application/x-custom;param="""#).unwrap();
        assert_eq!(mime.get_param("param").unwrap(), "");
    }

    #[test]
    fn test_mime_param_with_tab() {
        let mime = MediaType::from_str("application/x-custom;param=\"\t\"").unwrap();
        assert_eq!(mime.get_param("param").unwrap(), "\t");
    }

    #[test]
    fn test_mime_param_with_quoted_tab() {
        let mime = MediaType::from_str("application/x-custom;param=\"\\\t\"").unwrap();
        assert_eq!(mime.get_param("param").unwrap(), "\t");
    }

    #[test]
    fn test_reject_tailing_half_quoted_pair() {
        let mime = MediaType::from_str(r#"application/x-custom;param="\""#);
        assert!(mime.is_err());
    }

    #[test]
    fn test_parameter_eq_is_order_independent() {
        let mime_a = MediaType::from_str(r#"application/x-custom; param1=a; param2=b"#).unwrap();
        let mime_b = MediaType::from_str(r#"application/x-custom; param2=b; param1=a"#).unwrap();
        assert_eq!(mime_a, mime_b);
    }

    #[test]
    fn test_parameter_eq_is_order_independent_with_str() {
        let mime_a = MediaType::from_str(r#"application/x-custom; param1=a; param2=b"#).unwrap();
        let mime_b = r#"application/x-custom; param2=b; param1=a"#;
        assert_eq!(mime_a, mime_b);
    }

    #[test]
    fn test_name_eq_is_case_insensitive() {
        let mime1 = MediaType::from_str(r#"text/x-custom; abc=a"#).unwrap();
        let mime2 = MediaType::from_str(r#"text/x-custom; aBc=a"#).unwrap();
        assert_eq!(mime1, mime2);
    }

    #[test]
    fn test_media_type_parse_star_fails() {
        MediaType::from_str("*/*").expect_err("star/star");
        MediaType::from_str("image/*").expect_err("image/star");
        MediaType::from_str("text/*; charset=utf-8; q=0.9").expect_err("text/star;q");
    }
}

