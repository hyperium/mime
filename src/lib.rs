//! # Mime
//!
//! Mime is now Media Type, technically, but `Mime` is more immediately
//! understandable, so the main type here is `Mime`.
//!
//! ## What is Mime?
//!
//! Example mime string: `text/plain`
//!
//! ```
//! let plain_text: mime::Mime = "text/plain".parse().unwrap();
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

#![doc(html_root_url = "https://docs.rs/mime/0.3.6")]
#![deny(warnings)]
#![deny(missing_docs)]
#![deny(missing_debug_implementations)]


extern crate unicase;
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

/// A parsed mime or media type.
#[derive(Clone)]
pub struct Mime {
    source: Source,
    slash: usize,
    plus: Option<usize>,
    params: ParamSource,
}

/// An error when parsing a `Mime` from a string.
#[derive(Debug)]
pub struct FromStrError {
    inner: parse::ParseError,
}

impl Error for FromStrError {
    fn description(&self) -> &str {
        "an error occurred while parsing a MIME type"
    }
}

impl fmt::Display for FromStrError {
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

impl Mime {
    /// Get the top level media type for this `Mime`.
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
        Name {
            source: &self.source.as_ref()[..self.slash],
        }
    }

    /// Get the subtype of this `Mime`.
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
        let end = self.plus.unwrap_or_else(|| {
            self.semicolon().unwrap_or_else(|| self.source.as_ref().len())
        });
        Name {
            source: &self.source.as_ref()[self.slash + 1..end],
        }
    }

    /// Get an optional +suffix for this `Mime`.
    ///
    /// # Example
    ///
    /// ```
    /// let svg = "image/svg+xml".parse::<mime::Mime>().unwrap();
    /// assert_eq!(svg.suffix(), Some(mime::XML));
    /// assert_eq!(svg.suffix().unwrap(), "xml");
    ///
    ///
    /// assert!(mime::TEXT_PLAIN.suffix().is_none());
    /// ```
    #[inline]
    pub fn suffix(&self) -> Option<Name> {
        let end = self.semicolon().unwrap_or_else(|| self.source.as_ref().len());
        self.plus.map(|idx| Name {
            source: &self.source.as_ref()[idx + 1..end],
        })
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
    /// let mime = "multipart/form-data; boundary=ABCDEFG".parse::<mime::Mime>().unwrap();
    /// assert_eq!(mime.get_param(mime::BOUNDARY).unwrap(), "ABCDEFG");
    /// ```
    pub fn get_param<'a, N>(&'a self, attr: N) -> Option<Value<'a>>
    where N: PartialEq<Name<'a>> {
        self.params().find(|e| attr == e.0).map(|e| e.1)
    }

    /// Returns an iterator over the parameters.
    ///
    /// # Example
    ///
    /// ```
    /// let pkcs7: mime::Mime =
    ///     "application/pkcs7-mime; smime-type=enveloped-data; name=smime.p7m".parse().unwrap();
    ///
    /// let (names, values): (Vec<_>, Vec<_>) = pkcs7.params().unzip();
    ///
    /// assert_eq!(names, &["smime-type", "name"]);
    /// assert_eq!(values, &["enveloped-data", "smime.p7m"]);
    /// ```
    #[inline]
    pub fn params(&self) -> Params {
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

    /// Returns true if the media type has at last one parameter.
    ///
    /// # Example
    ///
    /// ```
    /// let plain_text: mime::Mime = "text/plain".parse().unwrap();
    /// assert_eq!(plain_text.has_params(), false);
    ///
    /// let plain_text_utf8: mime::Mime = "text/plain; charset=utf-8".parse().unwrap();
    /// assert_eq!(plain_text_utf8.has_params(), true);
    /// ```
    #[inline]
    pub fn has_params(&self) -> bool {
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

// Mime ============



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
                unicase::eq_ascii(self.source.as_ref(), s)
            } else {
                //OPTIMIZE: once the parser is rewritten and more modular
                // we can use parts of the parser to parse the string without
                // actually crating a mime, and use that for comparision
                s.parse::<Mime>()
                    .map(|other_mime| {
                        self == &other_mime
                    })
                    .unwrap_or(false)
            }
        } else if self.has_params() {
            s.parse::<Mime>()
                .map(|other_mime| {
                    self == &other_mime
                })
                .unwrap_or(false)
        } else {
            unicase::eq_ascii(self.source.as_ref(), s)
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

impl FromStr for Mime {
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<Mime, Self::Err> {
        parse::parse(s).map_err(|e| FromStrError { inner: e })
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
        let self_none = if let ParamsInner::None = self.0 { true } else { false };
        let other_none = if let ParamsInner::None = other.0 { true } else { false };
        if self_none && other_none {
            return FastEqRes::Equals;
        } else if self_none || other_none {
            return FastEqRes::NotEquals;
        }

        let self_utf8 = if let ParamsInner::Utf8 = self.0 { true } else { false };
        let other_utf8 = if let ParamsInner::Utf8 = other.0 { true } else { false };
        if self_utf8 && other_utf8 {
            return FastEqRes::Equals;
        }
        FastEqRes::Undetermined
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
    MARKDOWN, "markdown";
    LATEX, "latex";

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
/// # use mime::{Mime, CHARSET, UTF_8};
/// let mime = "text/plain; charset=utf-8".parse::<Mime>().unwrap();
/// assert_eq!(mime.get_param(CHARSET), Some(UTF_8));
/// ```
pub static UTF_8: Value = Value { source: "utf-8", ascii_case_insensitive: true };

macro_rules! mimes {
    ($($id:ident, $($piece:expr),*;)*) => (
        #[allow(non_camel_case_types)]
        enum __Atoms {
            __Dynamic,
        $(
            $id,
        )*
        }

        $(
            mime_constant! {
                $id, $($piece),*
            }
        )*

        #[test]
        fn test_mimes_macro_consts() {
            let _ = [
            $(
            mime_constant_test! {
                $id, $($piece),*
            }
            ),*
            ].iter().enumerate().map(|(pos, &atom)| {
                assert_eq!(pos + 1, atom as usize, "atom {} in position {}", atom, pos + 1);
            }).collect::<Vec<()>>();
        }
    )
}

macro_rules! mime_constant {
    ($id:ident, $src:expr, $slash:expr) => (
        mime_constant!($id, $src, $slash, None);
    );
    ($id:ident, $src:expr, $slash:expr, $plus:expr) => (
        mime_constant!(FULL $id, $src, $slash, $plus, ParamSource::None);
    );

    ($id:ident, $src:expr, $slash:expr, $plus:expr, $params:expr) => (
        mime_constant!(FULL $id, $src, $slash, $plus, ParamSource::Utf8($params));
    );


    (FULL $id:ident, $src:expr, $slash:expr, $plus:expr, $params:expr) => (
        #[doc = "`"]
        #[doc = $src]
        #[doc = "`"]
        pub const $id: Mime = Mime {
            source: Source::Atom(__Atoms::$id as u8, $src),
            slash: $slash,
            plus: $plus,
            params: $params,
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
        if let Some(plus) = __mime.plus {
            let __c = __mime.as_ref().as_bytes()[plus];
            assert_eq!(__c, b'+', "{:?} has {:?} at plus position {:?}", __mime, __c as char, plus);
        } else {
            assert!(!__mime.as_ref().as_bytes().contains(&b'+'), "{:?} forgot plus", __mime);
        }
        if let ParamSource::Utf8(semicolon) = __mime.params {
            assert_eq!(__mime.as_ref().as_bytes()[semicolon], b';');
            assert_eq!(&__mime.as_ref()[semicolon..], "; charset=utf-8");
        } else if let ParamSource::None = __mime.params {
            assert!(!__mime.as_ref().as_bytes().contains(&b';'));
        } else {
            unreachable!();
        }
        __mime.atom()
    })
}


mimes! {
    STAR_STAR, "*/*", 1;

    TEXT_STAR, "text/*", 4;
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
    TEXT_MARKDOWN, "text/markdown", 4;
    TEXT_LATEX, "text/latex", 4;

    IMAGE_STAR, "image/*", 5;
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
    APPLICATION_VEGA_V2, "application/vnd.vega.v2+json", 11, Some(23);
    APPLICATION_VEGA_V3, "application/vnd.vega.v3+json", 11, Some(23);
    APPLICATION_VEGALITE_V1, "application/vnd.vegalite.v1+json", 11, Some(27);
    APPLICATION_VEGALITE_V2, "application/vnd.vegalite.v2+json", 11, Some(27);
    APPLICATION_VEGALITE_V3, "application/vnd.vegalite.v3+json", 11, Some(27);
    APPLICATION_PLOTY, "application/vnd.plotly.v1+json", 11, Some(25);
    APPLICATION_GEOJSON, "application/geo+json", 11, Some(15);
    APPLICATION_FASTA, "application/vnd.fasta.fasta", 11;

    MULTIPART_FORM_DATA, "multipart/form-data", 9;
}

#[deprecated(since="0.3.1", note="please use `TEXT_JAVASCRIPT` instead")]
#[doc(hidden)]
pub const TEXT_JAVSCRIPT: Mime = TEXT_JAVASCRIPT;


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
        let mime = Mime::from_str("text/html+xml").unwrap();
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
        let mime = Mime::from_str("text/html+xml").unwrap();
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
    fn test_mime_from_str() {
        assert_eq!(Mime::from_str("text/plain").unwrap(), TEXT_PLAIN);
        assert_eq!(Mime::from_str("TEXT/PLAIN").unwrap(), TEXT_PLAIN);
        assert_eq!(Mime::from_str("text/plain; charset=utf-8").unwrap(), TEXT_PLAIN_UTF_8);
        assert_eq!(Mime::from_str("text/plain;charset=\"utf-8\"").unwrap(), TEXT_PLAIN_UTF_8);

        // quotes + semi colon
        Mime::from_str("text/plain;charset=\"utf-8\"; foo=bar").unwrap();
        Mime::from_str("text/plain;charset=\"utf-8\" ; foo=bar").unwrap();

        let upper = Mime::from_str("TEXT/PLAIN").unwrap();
        assert_eq!(upper, TEXT_PLAIN);
        assert_eq!(upper.type_(), TEXT);
        assert_eq!(upper.subtype(), PLAIN);


        let extended = Mime::from_str("TEXT/PLAIN; CHARSET=UTF-8; FOO=BAR").unwrap();
        assert_eq!(extended, "text/plain; charset=utf-8; foo=BAR");
        assert_eq!(extended.get_param("charset").unwrap(), "utf-8");
        assert_eq!(extended.get_param("foo").unwrap(), "BAR");

        Mime::from_str("multipart/form-data; boundary=--------foobar").unwrap();

        // stars
        assert_eq!("*/*".parse::<Mime>().unwrap(), STAR_STAR);
        assert_eq!("image/*".parse::<Mime>().unwrap(), "image/*");
        assert_eq!("text/*; charset=utf-8".parse::<Mime>().unwrap(), "text/*; charset=utf-8");

        // parse errors
        Mime::from_str("f o o / bar").unwrap_err();
        Mime::from_str("text\n/plain").unwrap_err();
        Mime::from_str("text\r/plain").unwrap_err();
        Mime::from_str("text/\r\nplain").unwrap_err();
        Mime::from_str("text/plain;\r\ncharset=utf-8").unwrap_err();
        Mime::from_str("text/plain; charset=\r\nutf-8").unwrap_err();
        Mime::from_str("text/plain; charset=\"\r\nutf-8\"").unwrap_err();
    }

    #[test]
    fn test_case_sensitive_values() {
        let mime = Mime::from_str("multipart/form-data; charset=BASE64; boundary=ABCDEFG").unwrap();
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

        let mime = Mime::from_str("text/plain; charset=utf-8; foo=bar").unwrap();
        assert_eq!(mime.get_param(CHARSET).unwrap(), "utf-8");
        assert_eq!(mime.get_param("foo").unwrap(), "bar");
        assert_eq!(mime.get_param("baz"), None);


        let mime = Mime::from_str("text/plain;charset=\"utf-8\"").unwrap();
        assert_eq!(mime.get_param(CHARSET), Some(UTF_8));
    }

    #[test]
    fn test_mime_with_dquote_quoted_pair() {
        let mime = Mime::from_str(r#"application/x-custom; title="the \" char""#).unwrap();
        assert_eq!(mime.get_param("title").unwrap(), "the \" char");
    }

    #[test]
    fn test_params() {
        let mime = TEXT_PLAIN;
        let mut params = mime.params();
        assert_eq!(params.next(), None);

        let mime = Mime::from_str("text/plain; charset=utf-8; foo=bar").unwrap();
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

        let mime = Mime::from_str("text/plain; charset=utf-8").unwrap();
        assert_eq!(mime.has_params(), true);

        let mime = Mime::from_str("text/plain; charset=utf-8; foo=bar").unwrap();
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
        let mime = Mime::from_str(r#"application/x-custom; param="Straße""#).unwrap();
        assert_eq!(mime.get_param("param").unwrap(), "Straße");
    }

    #[test]
    fn test_mime_with_multiple_plus() {
        let mime = Mime::from_str(r#"application/x-custom+bad+suffix"#).unwrap();
        assert_eq!(mime.type_(), "application");
        assert_eq!(mime.subtype(), "x-custom+bad");
        assert_eq!(mime.suffix().unwrap(), "suffix");
    }

    #[test]
    fn test_mime_param_with_empty_quoted_string() {
        let mime = Mime::from_str(r#"application/x-custom;param="""#).unwrap();
        assert_eq!(mime.get_param("param").unwrap(), "");
    }

    #[test]
    fn test_mime_param_with_tab() {
        let mime = Mime::from_str("application/x-custom;param=\"\t\"").unwrap();
        assert_eq!(mime.get_param("param").unwrap(), "\t");
    }

    #[test]
    fn test_mime_param_with_quoted_tab() {
        let mime = Mime::from_str("application/x-custom;param=\"\\\t\"").unwrap();
        assert_eq!(mime.get_param("param").unwrap(), "\t");
    }

    #[test]
    fn test_reject_tailing_half_quoted_pair() {
        let mime = Mime::from_str(r#"application/x-custom;param="\""#);
        assert!(mime.is_err());
    }

    #[test]
    fn test_parameter_eq_is_order_independent() {
        let mime_a = Mime::from_str(r#"application/x-custom; param1=a; param2=b"#).unwrap();
        let mime_b = Mime::from_str(r#"application/x-custom; param2=b; param1=a"#).unwrap();
        assert_eq!(mime_a, mime_b);
    }

    #[test]
    fn test_parameter_eq_is_order_independent_with_str() {
        let mime_a = Mime::from_str(r#"application/x-custom; param1=a; param2=b"#).unwrap();
        let mime_b = r#"application/x-custom; param2=b; param1=a"#;
        assert_eq!(mime_a, mime_b);
    }

    #[test]
    fn test_name_eq_is_case_insensitive() {
        let mime1 = Mime::from_str(r#"text/x-custom; abc=a"#).unwrap();
        let mime2 = Mime::from_str(r#"text/x-custom; aBc=a"#).unwrap();
        assert_eq!(mime1, mime2);
    }
}

