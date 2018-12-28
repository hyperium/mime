use std::cmp::PartialEq;
use std::fmt;


/// A name section of a `Mime`.
///
/// For instance, for the Mime `image/svg+xml`, it contains 3 `Name`s,
/// `image`, `svg`, and `xml`.
///
/// In all cases, `Name`s are compared case insensitive.
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct Name<'a> {
    /// The underlying str slice, which is _required to be lowercase_.
    /// Comparisons between two Name instances expect this, as they
    /// have to use `derive(PartialEq)` to be usable in a pattern
    pub(crate) source: &'a str,
}


impl<'a> Name<'a> {
    /// Get the value of this `Name` as a string.
    ///
    /// Note that the borrow is not tied to `&self` but the `'a` lifetime, allowing the
    /// string to outlive `Name`. Alternately, there is an `impl<'a> From<Name<'a>> for &'a str`
    /// which isn't rendered by Rustdoc, that can be accessed using `str::from(name)` or `name.into()`.
    pub fn as_str(&self) -> &'a str {
        self.source
    }

}

impl<'a> PartialEq<str> for Name<'a> {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.source.eq_ignore_ascii_case(other)
    }
}

impl<'a, 'b> PartialEq<&'b str> for Name<'a> {
    #[inline]
    fn eq(&self, other: & &'b str) -> bool {
        self == *other
    }
}

impl<'a> PartialEq<Name<'a>> for str {
    #[inline]
    fn eq(&self, other: &Name<'a>) -> bool {
        other == self
    }
}

impl<'a, 'b> PartialEq<Name<'a>> for &'b str {
    #[inline]
    fn eq(&self, other: &Name<'a>) -> bool {
        other == self
    }
}

impl<'a> AsRef<str> for Name<'a> {
    #[inline]
    fn as_ref(&self) -> &str {
        self.source
    }
}

impl<'a> From<Name<'a>> for &'a str {
    #[inline]
    fn from(name: Name<'a>) -> &'a str {
        name.source
    }
}

impl<'a> fmt::Debug for Name<'a> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self.source, f)
    }
}

impl<'a> fmt::Display for Name<'a> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self.source, f)
    }
}


#[cfg(test)]
mod test {
    use std::str::FromStr;
    use super::Name;
    use super::super::MediaType;

    #[test]
    fn test_name_eq_str() {
        let param = Name { source: "ABC" };

        assert_eq!(param, param);
        assert_eq!(param, "ABC");
        assert_eq!("ABC", param);
        assert_eq!(param, "abc");
        assert_eq!("abc", param);
    }

    #[test]
    fn test_name_eq_name() {
        let mime1 = MediaType::from_str(r#"text/x-custom; abc=a"#).unwrap();
        let mime2 = MediaType::from_str(r#"text/x-custom; aBc=a"#).unwrap();
        let param1 = mime1.params().next().unwrap().0;
        let param2 = mime2.params().next().unwrap().0;

        assert_eq!(param1, param2);
    }
}
