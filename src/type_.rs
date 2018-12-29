use std::fmt;
use std::str::FromStr;

use mime_parse::Mime;

use crate::{Atoms, InvalidMime, Value};

/// A parsed MIME or media type.
///
/// # Note about wildcards (`*`)
///
/// A `MediaType` represents an exact format type. The HTTP `Accet` header
/// can include "media ranges", which can match multiple media types. Those
/// "media ranges" should be represented as `MediaRange`.
#[derive(Clone, PartialEq)]
pub struct MediaType {
    pub(super) mime: Mime,
}

impl MediaType {
    /// Parse a string as a `MediaType`.
    ///
    /// # Example
    ///
    /// ```
    /// let mt = mime::MediaType::parse("text/plain").unwrap();
    /// assert_eq!(mt, mime::TEXT_PLAIN);
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the source is not a valid media type.
    #[inline]
    pub fn parse(source: &str) -> Result<Self, InvalidMime> {
        source.parse()
    }

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
    pub fn type_(&self) -> &str {
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
    ///
    /// let svg = mime::IMAGE_SVG;
    /// assert_eq!(svg.subtype(), mime::SVG);
    /// assert_eq!(svg.subtype(), "svg+xml");
    /// ```
    #[inline]
    pub fn subtype(&self) -> &str {
        self.mime.subtype()
    }

    /// Get an optional +suffix for this `MediaType`.
    ///
    /// # Example
    ///
    /// ```
    /// let svg = mime::IMAGE_SVG;
    /// assert_eq!(svg.suffix(), Some(mime::XML));
    /// assert_eq!(svg.suffix(), Some("xml"));
    ///
    ///
    /// assert!(mime::TEXT_PLAIN.suffix().is_none());
    /// ```
    #[inline]
    pub fn suffix(&self) -> Option<&str> {
        self.mime.suffix()
    }

    /// Look up a parameter by name.
    ///
    /// # Example
    ///
    /// ```
    /// let mime = mime::TEXT_PLAIN_UTF_8;
    /// assert_eq!(mime.param(mime::CHARSET), Some(mime::UTF_8));
    /// assert_eq!(mime.param("charset").unwrap(), "utf-8");
    /// assert!(mime.param("boundary").is_none());
    ///
    /// let mime = "multipart/form-data; boundary=ABCDEFG".parse::<mime::MediaType>().unwrap();
    /// assert_eq!(mime.param(mime::BOUNDARY).unwrap(), "ABCDEFG");
    /// ```
    pub fn param<'a>(&'a self, attr: &str) -> Option<Value<'a>> {
        self.params().find(|e| attr == e.0).map(|e| e.1)
    }


    /// Returns an iterator over the parameters.
    ///
    /// # Example
    ///
    /// ```
    /// let pkcs7 = mime::MediaType::parse(
    ///     "application/pkcs7-mime; smime-type=enveloped-data; name=smime.p7m"
    /// ).unwrap();
    ///
    /// let mut params = pkcs7.params();
    ///
    /// let (name, value) = params.next().unwrap();
    /// assert_eq!(name, "smime-type");
    /// assert_eq!(value, "enveloped-data");
    ///
    /// let (name, value) = params.next().unwrap();
    /// assert_eq!(name, "name");
    /// assert_eq!(value, "smime.p7m");
    ///
    /// assert!(params.next().is_none());
    /// ```
    #[inline]
    pub fn params(&self) -> impl Iterator<Item = (&str, Value)> {
        self.mime.params().map(|(n, v)| {
            let value = Value::new(v).for_name(n);
            (n, value)
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
    /// ```
    /// mime::media_type!("TEXT/PLAIN");
    /// ```
    ///
    /// # Parameters
    ///
    /// ```compile_fail
    /// mime::media_type!("multipart/form-data; boundary=abcd; two=2");
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
        params: crate::private::ParamSource,
    ) -> Self {
        MediaType {
            mime: Mime {
                source: crate::Source::Atom(source),
                slash,
                plus,
                params,
            }
        }
    }

    #[cfg(test)]
    pub(super) fn test_assert_asterisks(&self) {
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

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_type_() {
        assert_eq!(TEXT_PLAIN.type_(), TEXT);
    }


    #[test]
    fn test_subtype() {
        assert_eq!(TEXT_PLAIN.subtype(), PLAIN);
        assert_eq!(TEXT_PLAIN_UTF_8.subtype(), PLAIN);
        let mime = MediaType::parse("text/html+xml").unwrap();
        assert_eq!(mime.subtype(), "html+xml");
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
        let mime = MediaType::parse("text/html+xml").unwrap();
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
        assert_eq!(MediaType::parse("text/plain").unwrap(), TEXT_PLAIN);
        assert_eq!(MediaType::parse("TEXT/PLAIN").unwrap(), TEXT_PLAIN);
        assert_eq!(MediaType::parse("text/plain; charset=utf-8").unwrap(), TEXT_PLAIN_UTF_8);
        assert_eq!(MediaType::parse("text/plain;charset=\"utf-8\"").unwrap(), TEXT_PLAIN_UTF_8);

        // quotes + semi colon
        MediaType::parse("text/plain;charset=\"utf-8\"; foo=bar").unwrap();
        MediaType::parse("text/plain;charset=\"utf-8\" ; foo=bar").unwrap();

        let upper = MediaType::parse("TEXT/PLAIN").unwrap();
        assert_eq!(upper, TEXT_PLAIN);
        assert_eq!(upper.type_(), TEXT);
        assert_eq!(upper.subtype(), PLAIN);


        let extended = MediaType::parse("TEXT/PLAIN; CHARSET=UTF-8; FOO=BAR").unwrap();
        assert_eq!(extended, "text/plain; charset=utf-8; foo=BAR");
        assert_eq!(extended.param("charset").unwrap(), "utf-8");
        assert_eq!(extended.param("foo").unwrap(), "BAR");

        MediaType::parse("multipart/form-data; boundary=--------foobar").unwrap();

        // parse errors
        MediaType::parse("f o o / bar").unwrap_err();
        MediaType::parse("text\n/plain").unwrap_err();
        MediaType::parse("text\r/plain").unwrap_err();
        MediaType::parse("text/\r\nplain").unwrap_err();
        MediaType::parse("text/plain;\r\ncharset=utf-8").unwrap_err();
        MediaType::parse("text/plain; charset=\r\nutf-8").unwrap_err();
        MediaType::parse("text/plain; charset=\"\r\nutf-8\"").unwrap_err();
    }

    #[test]
    fn test_case_sensitive_values() {
        let mime = MediaType::parse("multipart/form-data; charset=BASE64; boundary=ABCDEFG").unwrap();
        assert_eq!(mime.param(CHARSET).unwrap(), "bAsE64");
        assert_eq!(mime.param(BOUNDARY).unwrap(), "ABCDEFG");
        assert_eq!(mime.param(BOUNDARY).unwrap().as_str_repr(), "ABCDEFG");
        assert_ne!(mime.param(BOUNDARY).unwrap(), "abcdefg");
    }

    #[test]
    fn test_get_param() {
        assert_eq!(TEXT_PLAIN.param("charset"), None);
        assert_eq!(TEXT_PLAIN.param("baz"), None);

        assert_eq!(TEXT_PLAIN_UTF_8.param("charset"), Some(UTF_8));
        assert_eq!(TEXT_PLAIN_UTF_8.param("baz"), None);

        let mime = MediaType::parse("text/plain; charset=utf-8; foo=bar").unwrap();
        assert_eq!(mime.param(CHARSET).unwrap(), "utf-8");
        assert_eq!(mime.param("foo").unwrap(), "bar");
        assert_eq!(mime.param("baz"), None);


        let mime = MediaType::parse("text/plain;charset=\"utf-8\"").unwrap();
        assert_eq!(mime.param(CHARSET), Some(UTF_8));
    }

    #[test]
    fn test_mime_with_dquote_quoted_pair() {
        let mime = MediaType::parse(r#"application/x-custom; title="the \" char""#).unwrap();
        assert_eq!(mime.param("title").unwrap(), "the \" char");
    }

    #[test]
    fn test_params() {
        let mime = TEXT_PLAIN;
        let mut params = mime.params();
        assert_eq!(params.next(), None);

        let mime = MediaType::parse("text/plain; charset=utf-8; foo=bar").unwrap();
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

        let mime = MediaType::parse("text/plain; charset=utf-8").unwrap();
        assert_eq!(mime.has_params(), true);

        let mime = MediaType::parse("text/plain; charset=utf-8; foo=bar").unwrap();
        assert_eq!(mime.has_params(), true);
    }

    #[test]
    fn test_mime_with_utf8_values() {
        let mime = MediaType::parse(r#"application/x-custom; param="Straße""#).unwrap();
        assert_eq!(mime.param("param").unwrap(), "Straße");
    }

    #[test]
    fn test_mime_with_multiple_plus() {
        let mime = MediaType::parse(r#"application/x-custom+bad+suffix"#).unwrap();
        assert_eq!(mime.type_(), "application");
        assert_eq!(mime.subtype(), "x-custom+bad+suffix");
        assert_eq!(mime.suffix().unwrap(), "suffix");
    }

    #[test]
    fn test_mime_param_with_empty_quoted_string() {
        let mime = MediaType::parse(r#"application/x-custom;param="""#).unwrap();
        assert_eq!(mime.param("param").unwrap(), "");
    }

    #[test]
    fn test_mime_param_with_tab() {
        let mime = MediaType::parse("application/x-custom;param=\"\t\"").unwrap();
        assert_eq!(mime.param("param").unwrap(), "\t");
    }

    #[test]
    fn test_mime_param_with_quoted_tab() {
        let mime = MediaType::parse("application/x-custom;param=\"\\\t\"").unwrap();
        assert_eq!(mime.param("param").unwrap(), "\t");
    }

    #[test]
    fn test_reject_tailing_half_quoted_pair() {
        let mime = MediaType::parse(r#"application/x-custom;param="\""#);
        assert!(mime.is_err());
    }

    #[test]
    fn test_parameter_eq_is_order_independent() {
        let mime_a = MediaType::parse(r#"application/x-custom; param1=a; param2=b"#).unwrap();
        let mime_b = MediaType::parse(r#"application/x-custom; param2=b; param1=a"#).unwrap();
        assert_eq!(mime_a, mime_b);
    }

    #[test]
    fn test_parameter_eq_is_order_independent_with_str() {
        let mime_a = MediaType::parse(r#"application/x-custom; param1=a; param2=b"#).unwrap();
        let mime_b = r#"application/x-custom; param2=b; param1=a"#;
        assert_eq!(mime_a, mime_b);
    }

    #[test]
    fn test_name_eq_is_case_insensitive() {
        let mime1 = MediaType::parse(r#"text/x-custom; abc=a"#).unwrap();
        let mime2 = MediaType::parse(r#"text/x-custom; aBc=a"#).unwrap();
        assert_eq!(mime1, mime2);
    }

    #[test]
    fn test_media_type_parse_star_fails() {
        MediaType::parse("*/*").expect_err("star/star");
        MediaType::parse("image/*").expect_err("image/star");
        MediaType::parse("text/*; charset=utf-8; q=0.9").expect_err("text/star;q");
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
        assert_eq!(svg.subtype(), "svg+xml");
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
        assert_eq!(utf8.param(CHARSET), Some(UTF_8));
        assert_eq!(utf8, TEXT_PLAIN_UTF_8);
    }

    #[cfg(feature = "macro")]
    #[test]
    fn test_media_type_macro_one_param() {
        let mt = media_type!("multipart/form-data; boundary=AbCd");
        assert_eq!(mt.type_(), MULTIPART);
        assert_eq!(mt.subtype(), FORM_DATA);
        assert_eq!(mt.suffix(), None);
        assert_eq!(mt.param("boundary").unwrap(), "AbCd");
    }

    #[cfg(feature = "macro")]
    #[test]
    fn test_media_type_macro_lowercase() {
        let mt = media_type!("MULTIPART/FORM-DATA; BOUNDARY=AbCd");
        assert_eq!(mt.to_string(), "multipart/form-data; boundary=AbCd");
    }
}

