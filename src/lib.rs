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
//! # const IGNORE_TOKENS: &str = stringify! {
//! let plain_text = mime::media_type!("text/plain");
//! # };
//! # let plain_text = mime::TEXT_PLAIN;
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

use mime_parse::{InternParams, Source, ParamSource};

#[cfg(feature = "macro")]
use proc_macro_hack::proc_macro_hack;

#[cfg(feature = "macro")]
#[proc_macro_hack]
pub use mime_macro::media_type;

pub use self::name::Name;
pub use self::range::MediaRange;
pub use self::type_::MediaType;
pub use self::value::{Value, UTF_8};

mod name;
mod range;
mod type_;
mod value;



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
        pub const $id: Name<'static> = Name::constant($e);
        )*

        #[test]
        fn test_names_macro_consts() {
            $(
            assert_eq!($id.as_ref().to_ascii_lowercase(), $id.as_ref());
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
    TAB_SEPARATED_VALUES, "tab-separated-values";

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
            mime: mime_parse::Mime {
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
        match __mime.mime.params {
            ParamSource::None | ParamSource::Utf8(_) => {
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
            },
            _ => (),
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
    fn intern(s: &str, slash: usize, params: InternParams) -> Source {
        debug_assert!(
            s.len() > slash,
            "intern called with illegal slash position: {:?}[{:?}]",
            s,
            slash,
        );

        match params {
            InternParams::Utf8(semicolon) => {
                Atoms::intern_charset_utf8(s, slash, semicolon)
            },
            InternParams::None => {
                Atoms::intern_no_params(s, slash)
            },
        }
    }

    fn intern_charset_utf8(s: &str, slash: usize, semicolon: usize) -> Source {
        let top = &s[..slash];
        let sub = &s[slash + 1..semicolon];

        if top == TEXT {
            if sub == PLAIN {
                return Atoms::TEXT_PLAIN_UTF_8;
            }
            if sub == HTML {
                return Atoms::TEXT_HTML_UTF_8;
            }
            if sub == CSS {
                return Atoms::TEXT_CSS_UTF_8;
            }
            if sub == CSV {
                return Atoms::TEXT_CSV_UTF_8;
            }
            if sub == TAB_SEPARATED_VALUES {
                return Atoms::TEXT_TAB_SEPARATED_VALUES_UTF_8;
            }
        }
        if top == APPLICATION {
            if sub == JAVASCRIPT {
                return Atoms::APPLICATION_JAVASCRIPT_UTF_8;
            }
        }

        Atoms::dynamic(s)
    }

    fn intern_no_params(s: &str, slash: usize) -> Source {
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
                            if sub == TAB_SEPARATED_VALUES {
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
                            if sub == Name::new("dns-message") {
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

        Atoms::dynamic(s)
    }

    fn dynamic(s: &str) -> Source {
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
