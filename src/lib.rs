#![doc(html_root_url = "https://docs.rs/mime/0.3.6")]
#![deny(warnings)]
#![deny(missing_docs)]
#![deny(missing_debug_implementations)]

//! # MediaType and MediaRange
//!
//! The `mime` crate defines two major types for representing MIMEs in HTTP
//! contexts:
//!
//! - A [`MediaType`](MediaType) is a concrete description of some content,
//!   such as `text/plain`.
//! - A [`MediaRange`](MediaRange) is a range of types that an agent is willing
//!   to receive, such as `text/*`.
//!
//! ## Getting a `MediaType`
//!
//! There are several constants exported for common media types:
//!
//! ```
//! let text = mime::TEXT_PLAIN;
//! let svg = mime::IMAGE_SVG;
//! let json = mime::APPLICATION_JSON;
//! // etc
//! ```
//!
//! A [`MediaType`](MediaType) can also be parsed from a string, such as from
//! a `Content-Type` HTTP header:
//!
//! ```
//! match mime::MediaType::parse("text/plain; charset=utf-8") {
//!     Ok(text) => assert_eq!(text, mime::TEXT_PLAIN_UTF_8),
//!     Err(err) => panic!("you should handle this parse error: {}", err),
//! }
//! ```
//!
//! ## Inspecting `MediaType`s
//!
//! Once you have a `MediaType`, you can inspect the various parts of it.
//! Since the `type_()` and `subtype()` methods return `&str`, you can make
//! easy-to-read `match` statements to handle different media types. To prevent
//! typos, many common type names are available as constants.
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
//! [`MediaRange`](MediaRange)s are often used by agents to declare a "range"
//! of media types that they can understand. A common place to find these is
//! `Accept` HTTP header, perhaps like this:
//!
//! ```http
//! GET /index.html HTTP/1.1
//! Accept: text/html, text/*
//! ```
//!
//! These can be parsed as `MediaRange`s, and then used to check if any of
//! the `MediaType`s you have would satisfy them.
//!
//! ```
//! match mime::MediaRange::parse("text/*") {
//!     Ok(range) => {
//!         // There's a couple constants in case you don't need parsing...
//!         assert_eq!(range, mime::TEXT_STAR);
//!
//!         // "text/plain" is a match
//!         assert!(range.matches(&mime::TEXT_PLAIN));
//!
//!         // "application/json" is NOT
//!         assert!(!range.matches(&mime::APPLICATION_JSON));
//!
//!     },
//!     Err(err) => panic!("that's a bad range: {}", err),
//! }
//! ```
#[cfg(feature = "macro")]
use proc_macro_hack::proc_macro_hack;

/// Compile-time `MediaType`s.
///
/// Performs validation and construction of a `MediaType` at compile-time,
/// catching parse errors early, and allowing them to be used as constants
/// or statics.
///
/// This requires the `macro` feature enabled on the mime crate. Something
/// like this in your `Cargo.toml`:
///
/// ```toml
/// [dependencies]
/// mime = { version = "0.4", features = ["macro"] }
/// ```
///
/// # Example
///
/// ```
/// const VND_MYAPP: mime::MediaType = mime::media_type!("application/vnd.myapp+json");
/// ```
#[cfg(feature = "macro")]
#[proc_macro_hack]
pub use mime_macro::media_type;

pub use mime_parse::constants::names::*;
pub use self::constants::mimes::*;
pub use self::error::InvalidMime;
pub use self::range::MediaRange;
pub use self::type_::MediaType;
pub use self::value::{Value, UTF_8};

mod cmp;
mod constants;
mod error;
#[cfg(feature = "macro")]
mod macros;
mod range;
#[cfg(feature = "serde1")]
mod serde;
mod type_;
mod value;


fn _assert_traits() {
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<InvalidMime>();
    assert_send_sync::<MediaRange>();
    assert_send_sync::<MediaType>();
    assert_send_sync::<Value>();
}

/// **DO NOT IMPORT THIS MODULE OR ITS TYPES.**
///
/// There is zero backwards-compatibility guarantee, your code *will* break.
/// Kittens **will** die.
#[doc(hidden)]
#[cfg(feature = "macro")]
pub mod private {
    #[doc(hidden)]
    pub use mime_parse::{Mime, ParamSource, Source};
}

#[cfg_attr(not(debug_assertions), allow(unused))]
fn is_ascii_lowercase(s: &str) -> bool {
    !s.as_bytes().iter().any(u8::is_ascii_uppercase)
}
