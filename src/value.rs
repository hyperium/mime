use std::cmp::PartialEq;
use std::fmt;
use std::borrow::Cow;

use mime_parse::Mime;
use quoted_string::{self, ContentChars, AsciiCaseInsensitiveEq};

/// a `Value` usable for a charset parameter.
///
/// # Example
/// ```
/// let mime = mime::TEXT_PLAIN_UTF_8;
/// assert_eq!(mime.param(mime::CHARSET), Some(mime::UTF_8));
/// ```
pub const UTF_8: Value = Value {
    source: "utf-8",
    ascii_case_insensitive: true,
};

/// A parameter value section of a `MediaType` or `MediaRange`.
///
/// Except for the `charset` parameter, parameters are compared case-sensitive.
#[derive(Clone, Copy)]
pub struct Value<'a> {
    source: &'a str,
    ascii_case_insensitive: bool,
}

pub(crate) fn params(mime: &Mime) -> impl Iterator<Item = (&str, Value)> {
    mime.params().map(|(n, v)| {
        let value = Value::new(v).for_name(n);
        (n, value)
    })
}

pub(crate) fn param<'a>(mime: &'a Mime, key: &str) -> Option<Value<'a>> {
    params(mime).find(|e| key == e.0).map(|e| e.1)
}

impl<'a> Value<'a> {
    fn new(source: &'a str) -> Self {
        Value {
            source,
            ascii_case_insensitive: false,
        }
    }

    fn for_name(mut self, name: &str) -> Self {
        debug_assert!(crate::is_ascii_lowercase(name));
        self.ascii_case_insensitive = name == crate::CHARSET;
        self
    }

    /// Returns the underlying representation.
    ///
    /// The underlying representation differs from the content,
    /// as it can contain quotes surrounding the content and
    /// quoted-pairs, even if non of them are necessary to
    /// represent the content.
    ///
    /// For example the representation `r#""a\"\ b""#` corresponds
    /// to the content `r#""a" b"#`. Another semantically  equivalent
    /// (i.e. with the same content) representation  is `r#""a\" b""`
    ///
    /// # Example
    ///
    /// ```
    /// let mime = r#"text/plain; param="abc def""#.parse::<mime::MediaType>().unwrap();
    /// let param = mime.param("param").unwrap();
    /// assert_eq!(param.as_str_repr(), r#""abc def""#);
    /// ```
    pub fn as_str_repr(&self) -> &'a str {
        self.source
    }

    /// Returns the content of this instance.
    ///
    /// It differs to the representation in that it will remove the
    /// quotation marks from the quoted string and will "unquote"
    /// quoted pairs.
    ///
    /// If the underlying representation is a quoted string containing
    /// quoted-pairs `Cow::Owned` is returned.
    ///
    /// If the underlying representation is a quoted-string without
    /// quoted-pairs `Cow::Borrowed` is returned as normal
    /// str slicing can be used to strip the surrounding double quoted.
    ///
    /// If the underlying representation is not a quoted-string
    /// `Cow::Borrowed` is returned, too.
    ///
    /// # Example
    ///
    /// ```
    /// let raw_mime = r#"text/plain; p1="char is \""; p2="simple"; p3=simple2"#;
    /// let mime = raw_mime.parse::<mime::MediaType>().unwrap();
    ///
    /// let param1 = mime.param("p1").unwrap();
    /// let expected = r#"char is ""#;
    /// assert_eq!(param1.to_content(), expected);
    ///
    /// let param2 = mime.param("p2").unwrap();
    /// assert_eq!(param2.to_content(), "simple");
    ///
    /// let param3 = mime.param("p3").unwrap();
    /// assert_eq!(param3.to_content(), "simple2");
    /// ```
    ///
    pub fn to_content(&self) -> Cow<'a, str> {
        quoted_string::unquote_unchecked(self.source)
    }

}

impl<'a, 'b> PartialEq<Value<'b>> for Value<'a> {
    #[inline]
    fn eq(&self, other: &Value<'b>) -> bool {
        let left_content_chars = ContentChars::from_string_unchecked(self.source);
        let right_content_chars = ContentChars::from_string_unchecked(other.source);

        if self.ascii_case_insensitive || other.ascii_case_insensitive {
            left_content_chars.eq_ignore_ascii_case(&right_content_chars)
        } else {
            left_content_chars == right_content_chars
        }
    }
}

impl<'a> PartialEq<str> for Value<'a> {
    fn eq(&self, other: &str) -> bool {
        if self.source.starts_with('"') {
            let content_chars = ContentChars::from_string_unchecked(self.source);
            if self.ascii_case_insensitive {
                content_chars.eq_ignore_ascii_case(other)
            } else {
                content_chars == other
            }
        } else if self.ascii_case_insensitive {
            self.source.eq_ignore_ascii_case(other)
        } else {
            self.source == other
        }
    }
}

impl<'a, 'b> PartialEq<&'b str> for Value<'a> {
    #[inline]
    fn eq(&self, other: & &'b str) -> bool {
        self == *other
    }
}


impl<'a, 'b> PartialEq<Value<'b>> for &'a str {
    #[inline]
    fn eq(&self, other: &Value<'b>) -> bool {
        other == self
    }
}

impl<'a> PartialEq<Value<'a>> for str {
    #[inline]
    fn eq(&self, other: &Value<'a>) -> bool {
        other == self
    }
}

impl<'a> From<Value<'a>> for Cow<'a, str> {
    #[inline]
    fn from(value: Value<'a>) -> Self {
        value.to_content()
    }
}

impl<'a> fmt::Debug for Value<'a> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self.source, f)
    }
}

impl<'a> fmt::Display for Value<'a> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self.source, f)
    }
}

#[cfg(test)]
mod test {
    use std::borrow::Cow;
    use std::cmp::PartialEq;
    use std::fmt::Debug;

    use super::Value;

    fn bidi_eq<A: Debug+PartialEq<B>, B: Debug+PartialEq<A>>(left: A, right: B) {
        assert_eq!(left, right);
        assert_eq!(right, left);
    }

    fn bidi_ne<A: Debug+PartialEq<B>, B: Debug+PartialEq<A>>(left: A, right: B) {
        assert_ne!(left, right);
        assert_ne!(right, left);
    }

    #[test]
    fn test_value_eq_str() {
        let value = Value {
            source: "abc",
            ascii_case_insensitive: false
        };
        let value_quoted = Value {
            source: "\"abc\"",
            ascii_case_insensitive: false
        };
        let value_quoted_with_esacpes = Value {
            source: "\"a\\bc\"",
            ascii_case_insensitive: false
        };

        bidi_eq(value, "abc");
        bidi_ne(value, "\"abc\"");
        bidi_ne(value, "\"a\\bc\"");

        bidi_eq(value_quoted, "abc");
        bidi_ne(value_quoted, "\"abc\"");
        bidi_ne(value_quoted, "\"a\\bc\"");

        bidi_eq(value_quoted_with_esacpes, "abc");
        bidi_ne(value_quoted_with_esacpes, "\"abc\"");
        bidi_ne(value_quoted_with_esacpes, "\"a\\bc\"");


        assert_ne!(value, "aBc");
        assert_ne!(value_quoted, "aBc");
        assert_ne!(value_quoted_with_esacpes, "aBc");
    }

    #[test]
    fn test_value_eq_str_ascii_case_insensitive() {
        let value = Value {
            source: "abc",
            ascii_case_insensitive: true
        };
        let value_quoted = Value {
            source: "\"abc\"",
            ascii_case_insensitive: true
        };
        let value_quoted_with_esacpes = Value {
            source: "\"a\\bc\"",
            ascii_case_insensitive: true
        };

        //1st. all case sensitive checks which still apply
        bidi_eq(value, "abc");
        bidi_ne(value, "\"abc\"");
        bidi_ne(value, "\"a\\bc\"");

        bidi_eq(value_quoted, "abc");
        bidi_ne(value_quoted, "\"abc\"");
        bidi_ne(value_quoted, "\"a\\bc\"");

        bidi_eq(value_quoted_with_esacpes, "abc");
        bidi_ne(value_quoted_with_esacpes, "\"abc\"");
        bidi_ne(value_quoted_with_esacpes, "\"a\\bc\"");


        //2nd the case insensitive check
        bidi_eq(value, "aBc");
        bidi_ne(value, "\"aBc\"");
        bidi_ne(value, "\"a\\Bc\"");

        bidi_eq(value_quoted, "aBc");
        bidi_ne(value_quoted, "\"aBc\"");
        bidi_ne(value_quoted, "\"a\\Bc\"");

        bidi_eq(value_quoted_with_esacpes, "aBc");
        bidi_ne(value_quoted_with_esacpes, "\"aBc\"");
        bidi_ne(value_quoted_with_esacpes, "\"a\\Bc\"");
    }

    #[test]
    fn test_value_eq_value() {
        let value = Value {
            source: "abc",
            ascii_case_insensitive: false
        };
        let value_quoted = Value {
            source: "\"abc\"",
            ascii_case_insensitive: false
        };
        let value_quoted_with_esacpes = Value {
            source: "\"a\\bc\"",
            ascii_case_insensitive: false
        };
        assert_eq!(value, value);
        assert_eq!(value_quoted, value_quoted);
        assert_eq!(value_quoted_with_esacpes, value_quoted_with_esacpes);

        bidi_eq(value, value_quoted);
        bidi_eq(value, value_quoted_with_esacpes);
        bidi_eq(value_quoted, value_quoted_with_esacpes);
    }

    #[test]
    fn test_value_eq_value_case_insensitive() {
        let value = Value {
            source: "Abc",
            ascii_case_insensitive: true
        };
        let value_quoted = Value {
            source: "\"aBc\"",
            ascii_case_insensitive: true
        };
        let value_quoted_with_esacpes = Value {
            source: "\"a\\bC\"",
            ascii_case_insensitive: true
        };
        assert_eq!(value, value);
        assert_eq!(value_quoted, value_quoted);
        assert_eq!(value_quoted_with_esacpes, value_quoted_with_esacpes);

        bidi_eq(value, value_quoted);
        bidi_eq(value, value_quoted_with_esacpes);
        bidi_eq(value_quoted, value_quoted_with_esacpes);
    }

    #[test]
    fn test_value_eq_value_mixed_case_sensitivity() {
        let value = Value {
            source: "Abc",
            ascii_case_insensitive: true
        };
        let value_quoted = Value {
            source: "\"aBc\"",
            ascii_case_insensitive: false
        };
        let value_quoted_with_esacpes = Value {
            source: "\"a\\bC\"",
            ascii_case_insensitive: false
        };

        bidi_eq(value, value_quoted);
        bidi_eq(value, value_quoted_with_esacpes);

        //both are ascii case insensitive
        bidi_ne(value_quoted, value_quoted_with_esacpes);
    }

    #[test]
    fn test_as_str_repr() {
        let value = Value::new("\"ab cd\"");
        assert_eq!(value, "ab cd");
        assert_eq!(value.as_str_repr(), "\"ab cd\"");
    }

    #[test]
    fn test_to_content_not_quoted() {
        let value = Value::new("abc");
        assert_eq!(value.to_content(), Cow::Borrowed("abc"));
    }

    #[test]
    fn test_to_content_quoted_simple() {
        let value = Value::new("\"ab cd\"");
        assert_eq!(value.to_content(), Cow::Borrowed("ab cd"));
    }

    #[test]
    fn test_to_content_with_quoted_pair() {
        let value = Value::new("\"ab\\\"cd\"");
        assert_eq!(value, "ab\"cd");
        let expected: Cow<'static, str> = Cow::Owned("ab\"cd".into());
        assert_eq!(value.to_content(), expected);
    }

}
