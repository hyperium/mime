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
//! let plain_text = mime::media_type!("text/plain");
//! assert_eq!(plain_text, mime::TEXT_PLAIN);
//! ```
//!
//! ## Inspecting Media Types
//!
//! ```
//! let mime = mime::TEXT_PLAIN;
//! match (mime.type_(), mime.subtype()) {
//!     (mime::TEXT, mime::PLAIN) => println!("plain text!"),
//!     (mime::TEXT, _) => println!("structured text"),
//!     _ => println!("not text"),
//! }
//! ```
//!
//! ## Using Media Ranges for matching
//!
//!
//! ```
//! assert!(mime::STAR_STAR.matches(&mime::TEXT_PLAIN));
//! assert!(mime::TEXT_STAR.matches(&mime::TEXT_PLAIN));
//! ```


use std::error::Error;
use std::fmt;
use std::str::FromStr;

use mime_parse::{Mime, Source, ParamSource};

#[cfg(feature = "macro")]
use proc_macro_hack::proc_macro_hack;

#[cfg(feature = "macro")]
#[proc_macro_hack]
pub use mime_macro::media_type;

pub use self::name::Name;
pub use self::value::Value;

mod name;
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

/// An invalid `MediaType` or `MediaRange`.
#[derive(Debug)]
pub struct InvalidMime {
    inner: mime_parse::ParseError,
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
        Name {
            source: self.mime.type_(),
        }
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
        Name {
            source: self.mime.subtype(),
        }
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
        self.mime.suffix().map(|source| Name { source })
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
        self.params().find(|e| attr == e.0).map(|e| e.1)
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
    pub fn params(&self) -> impl Iterator<Item = (Name, Value)> {
        self.mime.params().map(|(n, v)| {
            (
                Name { source: n },
                Value {
                    source: v,
                    ascii_case_insensitive: n == CHARSET,
                },
            )
        })
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

    /// **DO NOT CALL THIS FUNCTION.**
    ///
    /// This function has no backwards-compatibility guarantees. It can and
    /// *will* change, and your code *will* break.
    ///
    /// # Tests
    ///
    /// ```
    /// let foo = mime::media_type!("text/foo");
    /// assert_eq!(foo.type_(), mime::TEXT);
    /// assert_eq!(foo.subtype(), "foo");
    /// assert_eq!(foo.suffix(), None);
    /// assert!(!foo.has_params());
    /// ```
    ///
    /// # Uppercase
    ///
    /// ```compile_fail
    /// mime::media_type!("TEXT/PLAIN");
    /// ```
    ///
    /// # Parameters
    ///
    /// ```compile_fail
    /// mime::media_type!("multipart/form-data; boundary=abcd");
    /// ```
    ///
    /// # Ranges
    ///
    /// ```compile_fail
    /// mime::media_type!("text/*");
    /// ```
    ///
    /// # String literal
    ///
    /// ```compile_fail
    /// mime::media_type!(text/foo);
    /// ```
    ///
    /// ```compile_fail
    /// mime::media_type!("text/foo", "+json");
    /// ```
    ///
    /// # Dynamic Formatting
    ///
    /// ```compile_fail
    /// mime::media_type!("text/foo+{}", "json");
    /// ```
    #[doc(hidden)]
    #[cfg(feature = "macro")]
    pub const unsafe fn private_from_proc_macro(
        source: &'static str,
        slash: usize,
        plus: Option<usize>,
        params: ParamSource,
    ) -> Self {
        MediaType {
            mime: Mime {
                source: Source::Atom(source),
                slash,
                plus,
                params,
            }
        }
    }

    #[cfg(test)]
    fn test_assert_asterisks(&self) {
        assert!(!self.as_ref().contains('*'), "{:?} contains an asterisk", self);
    }
}

impl PartialEq<str> for MediaType {
    fn eq(&self, s: &str) -> bool {
        self.mime.eq_str(s, Atoms::intern)
    }
}

impl<'a> PartialEq<&'a str> for MediaType {
    #[inline]
    fn eq(&self, s: & &'a str) -> bool {
        self == *s
    }
}

impl<'a> PartialEq<MediaType> for &'a str {
    #[inline]
    fn eq(&self, mt: &MediaType) -> bool {
        mt == self
    }
}

impl PartialEq<MediaType> for str {
    #[inline]
    fn eq(&self, mt: &MediaType) -> bool {
        mt == self
    }
}

impl FromStr for MediaType {
    type Err = InvalidMime;

    fn from_str(s: &str) -> Result<MediaType, Self::Err> {
        mime_parse::parse(s, mime_parse::CanRange::No, Atoms::intern)
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
        Name {
            source: self.mime.type_(),
        }
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
        Name {
            source: self.mime.subtype(),
        }
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
        self.mime.suffix().map(|source| Name { source })

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
        self.params().find(|e| attr == e.0).map(|e| e.1)
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
    pub fn params(&self) -> impl Iterator<Item = (Name, Value)> {
        self.mime.params().map(|(n, v)| {
            (
                Name { source: n },
                Value {
                    source: v,
                    ascii_case_insensitive: n == CHARSET,
                },
            )
        })
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
        self.mime.eq_str(s, Atoms::intern)
    }
}

impl<'a> PartialEq<&'a str> for MediaRange {
    #[inline]
    fn eq(&self, s: & &'a str) -> bool {
        self == *s
    }
}

impl<'a> PartialEq<MediaRange> for &'a str {
    #[inline]
    fn eq(&self, mr: &MediaRange) -> bool {
        mr == self
    }
}

impl PartialEq<MediaRange> for str {
    #[inline]
    fn eq(&self, mr: &MediaRange) -> bool {
        mr == self
    }
}

impl FromStr for MediaRange {
    type Err = InvalidMime;

    fn from_str(s: &str) -> Result<MediaRange, Self::Err> {
        mime_parse::parse(s, mime_parse::CanRange::Yes, Atoms::intern)
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

/// **DO NOT IMPORT THIS MODULE OR ITS TYPES.**
///
/// There is zero backwards-compatibility guarantee, your code *will* break.
#[doc(hidden)]
#[cfg(feature = "macro")]
pub mod private {
    #[doc(hidden)]
    pub use mime_parse::ParamSource;
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
        $($(
            mime_constant! {
                $kind, $id, $($piece),+
            }
        )+)+


        #[test]
        fn test_mimes_macro_consts() {
            $($(
            mime_constant_test! {
                $id, $($piece),*
            }
            )+)+


            $($(
            mime_constant_proc_macro_test! {
                @$kind, $id, $($piece),*
            }
            )+)+
        }
    )
}

struct Atoms;

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

        impl Atoms {
            const $id: Source = Source::Atom($src);
        }

        #[doc = "`"]
        #[doc = $src]
        #[doc = "`"]
        pub const $id: $kind = $kind {
            mime: Mime {
                source: Atoms::$id,
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

        // check slash, plus, and semicolon are in correct positions
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
            unreachable!("consts wont have ParamSource::Custom");
        }


        // check that parsing can intern constants
        if let ParamSource::None = __mime.mime.params {
            let __parsed = mime_parse::parse($src, mime_parse::CanRange::Yes, Atoms::intern).expect("parse const");
            match __parsed.source {
                Source::Atom($src) => (),
                Source::Atom(src) => {
                    panic!(
                        "did not intern {:?} correctly: {:?}",
                        $src,
                        src,
                    );
                },
                _ => {
                    panic!(
                        "did not intern an Atom {:?}: slash={}, sub={}",
                        $src,
                        $slash,
                        $src.len() - $slash - 1,
                    );
                }
            }
        }

        // prevent ranges from being MediaTypes
        __mime.test_assert_asterisks();
    })
}

#[cfg(test)]
macro_rules! mime_constant_proc_macro_test {
    (@MediaType, $id:ident, $src:expr, $($unused:expr),+) => (
        // Test proc macro matches constants
        #[cfg(feature = "macro")]
        {
            let __mime = $id;
            let __m = media_type!($src);
            assert_eq!(__mime.mime.slash, __m.mime.slash);
            assert_eq!(__mime.mime.plus, __m.mime.plus);
            match __m.mime.source {
                Source::Atom($src) => (),
                Source::Atom(src) => {
                    panic!(
                        "did not intern {:?} correctly: {:?}",
                        $src,
                        src,
                    );
                },
                _ => {
                    panic!(
                        "did not intern an Atom {:?}",
                        $src,
                    );
                }
            }
        }
    );
    (@MediaRange, $id:ident, $src:expr, $($unused:expr),+) => ();
}


impl Atoms {
    fn intern(s: &str, slash: usize) -> Source {
        debug_assert!(
            s.len() > slash,
            "intern called with illegal slash position: {:?}[{:?}]",
            s,
            slash,
        );

        let top = &s[..slash];
        let sub = &s[slash + 1..];

        match slash {
            4 => {
                if top == TEXT {
                    match sub.len() {
                        1 => {
                            if sub.as_bytes()[0] == b'*' {
                                return Atoms::TEXT_STAR;
                            }
                        }
                        3 => {
                            if sub == CSS {
                                return Atoms::TEXT_CSS;
                            }
                            if sub == XML {
                                return Atoms::TEXT_XML;
                            }
                            if sub == CSV {
                                return Atoms::TEXT_CSV;
                            }
                        },
                        4 => {
                            if sub == HTML {
                                return Atoms::TEXT_HTML;
                            }
                        }
                        5 => {
                            if sub == PLAIN {
                                return Atoms::TEXT_PLAIN;
                            }
                            if sub == VCARD {
                                return Atoms::TEXT_VCARD;
                            }
                        }
                        10 => {
                            if sub == JAVASCRIPT {
                                return Atoms::TEXT_JAVASCRIPT;
                            }
                        }
                        12 => {
                            if sub == EVENT_STREAM {
                                return Atoms::TEXT_EVENT_STREAM;
                            }
                        },
                        20 => {
                            if sub == (Name { source: "tab-separated-values" }) {
                                return Atoms::TEXT_TAB_SEPARATED_VALUES;
                            }
                        }
                        _ => (),
                    }
                } else if top == FONT {
                    match sub.len() {
                        4 => {
                            if sub == WOFF {
                                return Atoms::FONT_WOFF;
                            }
                        },
                        5 => {
                            if sub == WOFF2 {
                                return Atoms::FONT_WOFF2;
                            }
                        },
                        _ => (),
                    }
                }
            },
            5 => {
                if top == IMAGE {
                    match sub.len() {
                        1 => {
                            if sub.as_bytes()[0] == b'*' {
                                return Atoms::IMAGE_STAR;
                            }
                        }
                        3 => {
                            if sub == PNG {
                                return Atoms::IMAGE_PNG;
                            }
                            if sub == GIF {
                                return Atoms::IMAGE_GIF;
                            }
                            if sub == BMP {
                                return Atoms::IMAGE_BMP;
                            }
                        }
                        4 => {
                            if sub == JPEG {
                                return Atoms::IMAGE_JPEG;
                            }
                        },
                        7 => {
                            if sub.as_bytes()[3] == b'+'
                                && &sub[..3] == SVG
                                && &sub[4..] == XML {
                                return Atoms::IMAGE_SVG;
                            }
                        },
                        _ => (),

                    }
                } else if top == VIDEO {
                    match sub.len() {
                        1 => {
                            if sub.as_bytes()[0] == b'*' {
                                return Atoms::VIDEO_STAR;
                            }
                        },
                        _ => (),
                    }
                } else if top == AUDIO {
                    match sub.len() {
                        1 => {
                            if sub.as_bytes()[0] == b'*' {
                                return Atoms::AUDIO_STAR;
                            }
                        },
                        _ => (),
                    }
                }
            },
            11 => {
                if top == APPLICATION {
                    match sub.len() {
                        3 => {
                            if sub == PDF {
                                return Atoms::APPLICATION_PDF;
                            }
                        }
                        4 => {
                            if sub == JSON {
                                return Atoms::APPLICATION_JSON;
                            }
                        },
                        7 => {
                            if sub == MSGPACK {
                                return Atoms::APPLICATION_MSGPACK;
                            }
                        },
                        10 => {
                            if sub == JAVASCRIPT {
                                return Atoms::APPLICATION_JAVASCRIPT;
                            }
                        },
                        11 => {
                            if sub == (Name { source: "dns-message" }) {
                                return Atoms::APPLICATION_DNS;
                            }
                        },
                        12 => {
                            if sub == OCTET_STREAM {
                                return Atoms::APPLICATION_OCTET_STREAM;
                            }
                        }
                        21 => {
                            if sub == WWW_FORM_URLENCODED {
                                return Atoms::APPLICATION_WWW_FORM_URLENCODED;
                            }
                        }
                        _ => (),
                    }
                }
            }
            _ => (),
        }

        Source::Dynamic(s.to_ascii_lowercase())
    }
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
        let any = "*/*".parse::<MediaRange>().unwrap();
        assert_eq!(any, "*/*");
        assert_eq!(any, STAR_STAR);
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

    #[cfg(feature = "macro")]
    #[test]
    fn test_media_type_macro_atom() {
        let a = media_type!("text/plain");
        let b = media_type!("text/plain");

        assert_eq!(a, TEXT_PLAIN);
        assert_eq!(b, TEXT_PLAIN);
        assert_eq!(a, b);
    }

    #[cfg(feature = "macro")]
    #[test]
    fn test_media_type_macro_custom() {
        let foo = media_type!("text/foo");
        assert_eq!(foo.type_(), TEXT);
        assert_eq!(foo.subtype(), "foo");
        assert_eq!(foo.suffix(), None);
        assert!(!foo.has_params());
    }

    #[cfg(feature = "macro")]
    #[test]
    fn test_media_type_macro_suffix() {
        let svg = media_type!("image/svg+xml");
        assert_eq!(svg.type_(), "image");
        assert_eq!(svg.subtype(), "svg");
        assert_eq!(svg.suffix(), Some(XML));
        assert!(!svg.has_params());
    }

    #[cfg(feature = "macro")]
    #[test]
    fn test_media_type_macro_utf8() {
        let utf8 = media_type!("text/plain; charset=utf-8");
        assert_eq!(utf8.type_(), TEXT);
        assert_eq!(utf8.subtype(), PLAIN);
        assert_eq!(utf8.suffix(), None);
        assert_eq!(utf8.get_param(CHARSET), Some(UTF_8));
        assert_eq!(utf8, TEXT_PLAIN_UTF_8);
    }

    /*
    #[cfg(feature = "macro")]
    #[test]
    fn test_media_type_macro_params() {
        let mt = media_type!("multipart/form-data; boundary=1234");
        assert_eq!(mt.type_(), MULTIPART);
        assert_eq!(mt.subtype(), FORM_DATA);
        assert_eq!(mt.suffix(), None);
        assert_eq!(mt.get_param("boundary").unwrap(), "1234");
    }
    */
}

