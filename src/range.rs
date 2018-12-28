use std::fmt;
use std::str::FromStr;

use mime_parse::Mime;

use crate::{Atoms, InvalidMime, MediaType, Name, Value};

/// A parsed media range used to match media types.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct MediaRange {
    pub(super) mime: Mime,
}

impl MediaRange {
    /// Parse a string as a `MediaRange`.
    ///
    /// # Example
    ///
    /// ```
    /// let range = mime::MediaRange::parse("*/*").unwrap();
    /// assert_eq!(range, mime::STAR_STAR);
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the source is not a valid media range.
    #[inline]
    pub fn parse(source: &str) -> Result<Self, InvalidMime> {
        source.parse()
    }

    /// Get the top level media type for this `MediaRange`.
    ///
    /// # Example
    ///
    /// ```
    /// let range = mime::TEXT_STAR;
    /// assert_eq!(range.type_(), "text");
    /// assert_eq!(range.type_(), mime::TEXT);
    /// ```
    #[inline]
    pub fn type_(&self) -> Name {
        Name::new(self.mime.type_())
    }

    /// Get the subtype of this `MediaRange`.
    ///
    /// # Example
    ///
    /// ```
    /// let range = mime::TEXT_STAR;
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
        Name::new(self.mime.subtype())
    }

    /// Get an optional +suffix for this `MediaRange`.
    ///
    /// # Example
    ///
    /// ```
    /// let svg = mime::MediaRange::from(mime::IMAGE_SVG);
    ///
    /// assert_eq!(svg.suffix(), Some(mime::XML));
    /// assert_eq!(svg.suffix().unwrap(), "xml");
    ///
    ///
    /// let any = mime::STAR_STAR;
    ///
    /// assert_eq!(any.suffix(), None);
    /// ```
    #[inline]
    pub fn suffix(&self) -> Option<Name> {
        self.mime.suffix().map(Name::new)
    }

    /// Checks if this `MediaRange` matches a specific `MediaType`.
    ///
    /// # Example
    ///
    /// ```
    /// let images = mime::IMAGE_STAR;
    ///
    /// assert!(images.matches(&mime::IMAGE_JPEG));
    /// assert!(images.matches(&mime::IMAGE_PNG));
    ///
    /// assert!(!images.matches(&mime::TEXT_PLAIN));
    /// ```
    pub fn matches(&self, mt: &MediaType) -> bool {
        let type_ = self.type_();

        if type_ == crate::STAR {
            // sanity check there's no `*/plain` or whatever
            debug_assert_eq!(self.subtype(), crate::STAR);

            return self.matches_params(mt);
        }

        if type_ != mt.type_() {
            return false;
        }

        let subtype = self.subtype();

        if subtype == crate::STAR {
            return self.matches_params(mt);
        }

        if subtype != mt.subtype() {
            return false;
        }

        // type and subtype are the same, last thing to do is check
        // that the MediaType contains all this range's parameters...
        self.matches_params(mt)
    }

    fn matches_params(&self, mt: &MediaType) -> bool {
        for (name, value) in self.params() {
            if name != "q" && mt.param(name) != Some(value) {
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
    /// let range = mime::MediaRange::from(mime::TEXT_PLAIN_UTF_8);
    ///
    /// assert_eq!(range.param(mime::CHARSET), Some(mime::UTF_8));
    /// assert_eq!(range.param("charset").unwrap(), "utf-8");
    /// assert_eq!(range.param("boundary"), None);
    /// ```
    pub fn param<'a, N>(&'a self, attr: N) -> Option<Value<'a>>
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
    /// let pkcs7 = mime::MediaRange::parse(
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
    pub fn params(&self) -> impl Iterator<Item = (Name, Value)> {
        self.mime.params().map(|(n, v)| {
            let name = Name::new(n);
            let value = Value::new(v).for_name(name);
            (name, value)
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
    pub(super) fn test_assert_asterisks(&self) {
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

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn media_range_from_str() {
        // exact types
        assert_eq!(MediaRange::parse("text/plain").unwrap(), MediaRange::from(TEXT_PLAIN));

        // stars
        let any = "*/*".parse::<MediaRange>().unwrap();
        assert_eq!(any, "*/*");
        assert_eq!(any, STAR_STAR);
        assert_eq!("image/*".parse::<MediaRange>().unwrap(), "image/*");
        assert_eq!("text/*; charset=utf-8".parse::<MediaRange>().unwrap(), "text/*; charset=utf-8");

        // bad stars
        MediaRange::parse("text/*plain").unwrap_err();
    }

    #[test]
    fn media_range_matches() {
        assert!(STAR_STAR.matches(&TEXT_PLAIN), "*/* matches everything");

        assert!(TEXT_STAR.matches(&TEXT_PLAIN), "text/* matches text/plain");
        assert!(TEXT_STAR.matches(&TEXT_HTML), "text/* matches text/html");
        assert!(TEXT_STAR.matches(&TEXT_HTML_UTF_8), "text/* matches text/html; charset=utf-8");

        assert!(!TEXT_STAR.matches(&IMAGE_GIF), "text/* doesn't match image/gif");
    }

    #[test]
    fn media_range_matches_params() {
        let text_any_utf8 = MediaRange::parse("text/*; charset=utf-8").unwrap();

        assert!(text_any_utf8.matches(&TEXT_PLAIN_UTF_8));
        assert!(text_any_utf8.matches(&TEXT_HTML_UTF_8));

        assert!(!text_any_utf8.matches(&TEXT_HTML));

        let many_params = MediaType::parse("text/plain; charset=utf-8; foo=bar").unwrap();
        assert!(text_any_utf8.matches(&many_params));
    }

    #[test]
    fn media_range_matches_skips_q() {
        let range = MediaRange::parse("text/*; q=0.8").unwrap();

        assert!(range.matches(&TEXT_PLAIN_UTF_8));
        assert!(range.matches(&TEXT_HTML_UTF_8));

        let range = MediaRange::parse("text/*; charset=utf-8; q=0.8").unwrap();

        assert!(range.matches(&TEXT_PLAIN_UTF_8));
        assert!(range.matches(&TEXT_HTML_UTF_8));
        assert!(!range.matches(&TEXT_HTML));
    }
}

