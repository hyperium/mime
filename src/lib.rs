//! # Mime
//!
//! Mime is now Media Type, technically, but `Mime` is more immediately
//! understandable, so the main type here is `Mime`.
//!
//! ## What is Mime?
//!
//! Example mime string: `text/plain;charset=utf-8`
//!
//! ```rust
//! # use std::from_str::FromStr;
//! use mime::{Mime, Text, Plain, Charset, Utf8};
//! let mime: Mime = FromStr::from_str("text/plain;charset=utf-8").unwrap();
//! assert_eq!(mime, Mime(Text, Plain, vec![(Charset, Utf8)]));
//! ```

#![license = "MIT"]
#![doc(html_root_url = "http://seanmonstar.github.io/mime.rs")]
#![experimental]
#![feature(macro_rules, phase)]

#[phase(plugin, link)]
extern crate log;

#[cfg(test)]
extern crate test;

use std::ascii::StrAsciiExt;
use std::cmp::Equiv;
use std::fmt;
use std::from_str::FromStr;
use std::iter::Enumerate;
use std::str::Chars;

macro_rules! inspect(
    ($s:expr, $t:expr) => ({
        let t = $t;
        debug!("inspect {}: {}", $s, t);
        t
    })
)

/// Mime, or Media Type. Encapsulates common registers types.
///
/// Consider that a traditional mime type contains a "top level type",
/// a "sub level type", and 0-N "parameters". And they're all strings.
/// Strings everywhere. Strings mean typos. Rust has type safety. We should
/// use types!
///
/// So, Mime bundles together this data into types so the compiler can catch
/// your typos.
///
/// This improves things so you use match without Strings:
///
/// ```rust
/// use std::from_str::from_str;
/// use mime::{Mime, Application, Json};
///
/// let mime: mime::Mime = from_str("application/json").unwrap();
///
/// match mime {
///     Mime(Application, Json, _) => println!("matched json!"),
///     _ => ()
/// }
/// ```
#[deriving(Clone, PartialEq)]
pub struct Mime(pub TopLevel, pub SubLevel, pub Vec<Param>);

macro_rules! enoom (
    (pub enum $en:ident; $ext:ident; $($ty:ident, $text:expr;)*) => (

        #[deriving(Clone, PartialEq)]
        pub enum $en {
            $($ty),*,
            $ext(String)
        }

        impl fmt::Show for $en {
            fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                match *self {
                    $($ty => $text),*,
                    $ext(ref s) => return s.fmt(fmt)
                }.fmt(fmt)
            }
        }

        impl FromStr for $en {
            fn from_str(s: &str) -> Option<$en> {
                Some(match s {
                    $(_s if _s == $text => $ty),*,
                    s => $ext(inspect!(stringify!($ext), s).to_string())
                })
            }
        }
    )
)

enoom! {
    pub enum TopLevel;
    TopExt;
    TopStar, "*"; // remove Top prefix if enums gain namespaces
    Text, "text";
    Image, "image";
    Audio, "audio";
    Video, "video";
    Application, "application";
    Multipart, "multipart";
    Message, "message";
    Model, "model";
}

enoom! {
    pub enum SubLevel;
    SubExt;
    SubStar, "*"; // remove Sub prefix if enums gain namespaces

    // common text/*
    Plain, "plain";
    Html, "html";
    Xml, "xml";
    Javascript, "javascript";
    Css, "css";
    
    // common application/*
    Json, "json";
    
    // common image/*
    Png, "png";
    Gif, "gif";
    Bmp, "bmp";
    Jpeg, "jpeg";
}

enoom! {
    pub enum Attr;
    AttrExt;
    Charset, "charset";
    Q, "q";
}

enoom! {
    pub enum Value;
    ValueExt;
    Utf8, "utf-8";
}

pub type Param = (Attr, Value);

impl Equiv<Mime> for Mime {
    fn equiv(&self, other: &Mime) -> bool {
        //im so sorry
        //TODO: be less sorry. dont to_string()
        self.to_string() == other.to_string()
    }
}

impl fmt::Show for Mime {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let Mime(ref top, ref sub, ref params) = *self;
        try!(write!(fmt, "{}/{}", top, sub));
        fmt_params(params.as_slice(), fmt)
    }
}

impl FromStr for Mime {
    fn from_str(raw: &str) -> Option<Mime> {
        let ascii = raw.to_ascii_lower(); // lifetimes :(
        let raw = ascii.as_slice();
        let len = raw.len();
        let mut iter = raw.chars().enumerate();
        let mut params = vec![];
        // toplevel
        let mut start;
        let mut top;
        loop {
            match inspect!("top iter", iter.next()) {
                Some((0, c)) if is_restricted_name_first_char(c) => (),
                Some((i, c)) if i > 0 && is_restricted_name_char(c) => (),
                Some((i, '/')) if i > 0 => match FromStr::from_str(raw.slice_to(i)) {
                    Some(t) => {
                        top = t;
                        start = i + 1;
                        break;
                    }
                    None => return None
                },
                _ => return None // EOF and no toplevel is no Mime
            };

        }

        // sublevel
        let mut sub;
        loop {
            match inspect!("sub iter", iter.next()) {
                Some((i, c)) if i == start && is_restricted_name_first_char(c) => (),
                Some((i, c)) if i > start && is_restricted_name_char(c) => (),
                Some((i, ';')) if i > start => match FromStr::from_str(raw.slice(start, i)) {
                    Some(s) => {
                        sub = s;
                        start = i + 1;
                        break;
                    }
                    None => return None
                },
                None => match FromStr::from_str(raw.slice_from(start)) {
                    Some(s) => return Some(Mime(top, s, params)),
                    None => return None
                },
                _ => return None
            };
        }

        // params
        debug!("starting params, len={}", len);
        loop {
            match inspect!("param", param_from_str(raw, &mut iter, start)) {
                Some((p, end)) => {
                    params.push(p);
                    start = end;
                    if start >= len {
                        break;
                    }
                }
                None => break
            }
        }

        Some(Mime(top, sub, params))
    }
}

fn param_from_str(raw: &str, iter: &mut Enumerate<Chars>, mut start: uint) -> Option<(Param, uint)> {
    let mut attr;
    debug!("param_from_str, start={}", start);
    loop {
        match inspect!("attr iter", iter.next()) {
            Some((i, ' ')) if i == start => start = i + 1,
            Some((i, c)) if i == start && is_restricted_name_first_char(c) => (),
            Some((i, c)) if i > start && is_restricted_name_char(c) => (),
            Some((i, '=')) if i > start => match FromStr::from_str(raw.slice(start, i)) {
                Some(a) => {
                    attr = inspect!("attr", a);
                    start = i + 1;
                    break;
                },
                None => return None
            },
            _ => return None
        }
    }
    let mut value;
    // values must be restrict-name-char or "anything goes"
    let mut is_quoted = false;
    loop {
        match inspect!("value iter", iter.next()) {
            Some((i, '"')) if i == start => {
                debug!("quoted");
                is_quoted = true;
                start = i + 1;
            },
            Some((i, c)) if i == start && is_restricted_name_first_char(c) => (),
            Some((i, '"')) if i > start && is_quoted => match FromStr::from_str(raw.slice(start, i)) {
                Some(v) => {
                    value = v;
                    start = i + 1;
                    break;
                },
                None => return None
            },
            Some((i, c)) if i > start && is_quoted || is_restricted_name_char(c) => (),
            Some((i, ';')) if i > start => match FromStr::from_str(raw.slice(start, i)) {
                Some(v) => {
                    value = v;
                    start = i + 1;
                    break;
                },
                None => return None
            },
            None => match FromStr::from_str(raw.slice_from(start)) {
                Some(v) => {
                    value = v;
                    start = raw.len();
                    break;
                },
                None => return None
            },

            _ => return None
        }
    }

    Some(((attr, value), start))
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
//
fn is_restricted_name_first_char(c: char) -> bool {
    match c {
        'a'...'z' |
        '0'...'9' => true,
        _ => false
    }
}

fn is_restricted_name_char(c: char) -> bool {
    if is_restricted_name_first_char(c) {
        true
    } else {
        match c {
            '!' |
            '#' |
            '$' |
            '&' |
            '-' |
            '^' |
            '.' |
            '+' |
            '_' => true,
            _ => false
        }
    }
}


#[inline]
fn fmt_params<T: AsSlice<Param>>(params: T, fmt: &mut fmt::Formatter) -> fmt::Result {
    for param in params.as_slice().iter() {
        try!(fmt_param(param, fmt));
    }
    Ok(())
}

#[inline]
fn fmt_param(param: &Param, fmt: &mut fmt::Formatter) -> fmt::Result {
    let (ref attr, ref value) = *param;
    write!(fmt, "; {}={}", attr, value)
}

#[cfg(test)]
mod tests {
    use std::from_str::{FromStr, from_str};
    use test::Bencher;
    use super::{Mime, Text, Plain, Charset, Utf8, AttrExt, ValueExt};

    #[test]
    fn test_mime_show() {
        let mime = Mime(Text, Plain, vec![]);
        assert_eq!(mime.to_string(), "text/plain".to_string());
        let mime = Mime(Text, Plain, vec![(Charset, Utf8)]);
        assert_eq!(mime.to_string(), "text/plain; charset=utf-8".to_string());
    }

    #[test]
    fn test_mime_from_str() {
        assert_eq!(FromStr::from_str("text/plain"), Some(Mime(Text, Plain, vec![])));
        assert_eq!(FromStr::from_str("TEXT/PLAIN"), Some(Mime(Text, Plain, vec![])));
        assert_eq!(FromStr::from_str("text/plain; charset=utf-8"), Some(Mime(Text, Plain, vec![(Charset, Utf8)])));
        assert_eq!(FromStr::from_str("text/plain;charset=\"utf-8\""), Some(Mime(Text, Plain, vec![(Charset, Utf8)])));
        assert_eq!(FromStr::from_str("text/plain; charset=utf-8; foo=bar"),
            Some(Mime(Text, Plain, vec![(Charset, Utf8),
                                        (AttrExt("foo".to_string()), ValueExt("bar".to_string())) ])));
    }


    #[bench]
    fn bench_show(b: &mut Bencher) {
        let mime = Mime(Text, Plain, vec![(Charset, Utf8), (AttrExt("foo".to_string()), ValueExt("bar".to_string()))]);
        b.bytes = mime.to_string().as_bytes().len() as u64;
        b.iter(|| mime.to_string())
    }

    #[bench]
    fn bench_from_str(b: &mut Bencher) {
        let s = "text/plain; charset=utf-8; foo=bar";
        b.bytes = s.as_bytes().len() as u64;
        b.iter(|| from_str::<Mime>(s))
    }
}
