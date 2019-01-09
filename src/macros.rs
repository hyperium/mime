use super::MediaType;

impl MediaType {
    /// **DO NOT CALL THIS FUNCTION.**
    ///
    /// This function has no backwards-compatibility guarantees. It can and
    /// *will* change, and your code *will* break.
    /// Kittens **will** die.
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
        mime: crate::private::Mime,
    ) -> Self {
        MediaType {
            mime,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn media_type_atom() {
        let a = media_type!("text/plain");
        let b = media_type!("text/plain");

        assert_eq!(a, TEXT_PLAIN);
        assert_eq!(b, TEXT_PLAIN);
        assert_eq!(a, b);
    }

    #[test]
    fn media_type_custom() {
        let foo = media_type!("text/foo");
        assert_eq!(foo.type_(), TEXT);
        assert_eq!(foo.subtype(), "foo");
        assert_eq!(foo.suffix(), None);
        assert!(!foo.has_params());

        let parsed = MediaType::parse("text/foo").unwrap();
        assert_eq!(foo, parsed);

        let foo2 = media_type!("text/foo");
        assert_eq!(foo, foo2);

        let bar = media_type!("text/bar");
        assert_ne!(foo, bar);
    }

    #[test]
    fn media_type_suffix() {
        let svg = media_type!("image/svg+xml");
        assert_eq!(svg.type_(), "image");
        assert_eq!(svg.subtype(), "svg+xml");
        assert_eq!(svg.suffix(), Some(XML));
        assert!(!svg.has_params());
    }

    #[test]
    fn media_type_atom_utf8() {
        let utf8 = media_type!("text/plain; charset=utf-8");
        assert_eq!(utf8.type_(), TEXT);
        assert_eq!(utf8.subtype(), PLAIN);
        assert_eq!(utf8.suffix(), None);
        assert_eq!(utf8.param(CHARSET), Some(UTF_8));
        assert_eq!(utf8, TEXT_PLAIN_UTF_8);
    }

    #[test]
    fn media_type_one_param() {
        let mt = media_type!("multipart/form-data; boundary=AbCd");
        assert_eq!(mt.type_(), MULTIPART);
        assert_eq!(mt.subtype(), FORM_DATA);
        assert_eq!(mt.suffix(), None);
        assert_eq!(mt.param("boundary").unwrap(), "AbCd");
    }

    #[test]
    fn media_type_lowercase() {
        let mt = media_type!("MULTIPART/FORM-DATA; BOUNDARY=AbCd");
        assert_eq!(mt.to_string(), "multipart/form-data; boundary=AbCd");
    }
}

