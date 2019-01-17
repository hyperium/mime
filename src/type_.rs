use std::fmt;
use std::str::FromStr;

use mime_parse::{Mime, Parse};

use crate::{InvalidMime, Value};

/// A parsed media type (or "MIME").
///
/// ## Getting a `MediaType`
///
/// There are several constants exported for common media types:
///
/// ```
/// let text = mime::TEXT_PLAIN;
/// let svg = mime::IMAGE_SVG;
/// let json = mime::APPLICATION_JSON;
/// // etc
/// ```
///
/// A `MediaType` can also be parsed from a string, such as from
/// a `Content-Type` HTTP header:
///
/// ```
/// match mime::MediaType::parse("text/plain; charset=utf-8") {
///     Ok(text) => assert_eq!(text, mime::TEXT_PLAIN_UTF_8),
///     Err(err) => panic!("you should handle this parse error: {}", err),
/// }
/// ```
///
/// ## Inspecting `MediaType`s
///
/// Once you have a `MediaType`, you can inspect the various parts of it.
/// Since the `type_()` and `subtype()` methods return `&str`, you can make
/// easy-to-read `match` statements to handle different media types. To prevent
/// typos, many common type names are available as constants.
///
/// ```
/// let mime = mime::TEXT_PLAIN;
/// match (mime.type_(), mime.subtype()) {
///     (mime::TEXT, mime::PLAIN) => println!("plain text!"),
///     (mime::TEXT, _) => println!("structured text"),
///     _ => println!("not text"),
/// }
/// ```
///
/// ## Note about wildcards (`*`)
///
/// A `MediaType` represents an exact format type. The HTTP `Accept` header
/// can include "media ranges", which can match multiple media types. Those
/// "media ranges" should be represented as [`MediaRange`](super::MediaRange).
#[derive(Clone)]
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
    pub fn parse(source: impl Parse) -> Result<Self, InvalidMime> {
        mime_parse::Parser::cannot_range()
            .parse(source)
            .map(|mime| MediaType { mime })
            .map_err(|e| InvalidMime { inner: e })
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
        crate::value::param(&self.mime, attr)
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
        crate::value::params(&self.mime)
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

    /// Transforms the media type into its non-parametrized form.
    ///
    /// # Example
    ///
    /// ```
    /// use mime::MediaType;
    ///
    /// let html_xml_utf8 = MediaType::parse("text/html+xml; charset=utf-8").unwrap();
    /// assert!(html_xml_utf8.has_params());
    ///
    /// let html_xml = html_xml_utf8.without_params();
    /// assert!(!html_xml.has_params());
    ///
    /// assert_eq!(html_xml, "text/html+xml");
    /// ```
    #[inline]
    pub fn without_params(mut self) -> Self {
        self.mime = self.mime.without_params();
        self
    }

    #[cfg(test)]
    pub(super) fn test_assert_asterisks(&self) {
        assert!(!self.as_ref().contains('*'), "{:?} contains an asterisk", self);
    }
}

impl PartialEq for MediaType {
    fn eq(&self, other: &MediaType) -> bool {
        crate::cmp::mime_eq(&self.mime, &other.mime)
    }
}

impl PartialEq<str> for MediaType {
    fn eq(&self, s: &str) -> bool {
        crate::cmp::str_eq(&self.mime, s)
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
        MediaType::parse(s)
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
    fn test_size_of() {
        assert!(
            std::mem::size_of::<MediaType>() < 100,
            "just to be warned if the size grows suddenly"
        );

        assert_eq!(
            std::mem::size_of::<MediaType>(),
            std::mem::size_of::<Option<MediaType>>(),
            "option size optimization"
        );
    }

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

        // empty quotes
        MediaType::parse("audo/wave; codecs=\"\"").expect("param value with empty quotes");

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
    fn test_from_str_empty_parameter_list() {
        static CASES: &'static [&'static str] = &[
            "text/event-stream;",
            "text/event-stream; ",
            "text/event-stream;       ",
        ];

        for &case in CASES {
            let mime = MediaType::parse(case).expect(case);
            assert_eq!(mime, TEXT_EVENT_STREAM, "case = {:?}", case);
            assert_eq!(mime.type_(), TEXT, "case = {:?}", case);
            assert_eq!(mime.subtype(), EVENT_STREAM, "case = {:?}", case);
            assert!(!mime.has_params(), "case = {:?}", case);
        }
    }

    #[test]
    fn test_parse_too_long() {
        let mut source = vec![b'a'; ::std::u16::MAX as usize];
        source[5] = b'/';

        let mut s = String::from_utf8(source).unwrap();

        MediaType::parse(&s).expect("parses AT max length");

        s.push('a');
        MediaType::parse(&s).expect_err("errors OVER max length");
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

    #[test]
    fn test_cmp_params_not_equal() {
        let mime1 = MediaType::parse("text/plain; aaa=bbb").unwrap();
        let mime2 = MediaType::parse("text/plain; ccc=ddd").unwrap();

        assert_ne!(mime1, mime2);
    }
}

