#![doc(html_root_url = "https://docs.rs/mime/0.3.6")]
#![deny(warnings)]
#![deny(missing_docs)]
#![deny(missing_debug_implementations)]

//! # MediaType and MediaRange
//!
//! The `mime` crate defines two major types for representing MIMEs:
//!
//! - A `MediaType` is a concrete description of some content, such as `text/plain`.
//! - A `MediaRange` is a range of types that an agent is willing to receive, such as `text/*`.
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
#[cfg(feature = "macro")]
use proc_macro_hack::proc_macro_hack;

#[cfg(feature = "macro")]
#[proc_macro_hack]
pub use mime_macro::media_type;

pub use mime_parse::constants::names::*;
pub use self::constants::mimes::*;
pub use self::error::InvalidMime;
pub use self::range::MediaRange;
pub use self::type_::MediaType;
pub use self::value::{Value, UTF_8};

mod constants;
mod error;
mod range;
mod type_;
mod value;



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
