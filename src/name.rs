use std::cmp::PartialEq;
use std::fmt;


use unicase;

/// A name section of a `Mime`.
///
/// For instance, for the Mime `image/svg+xml`, it contains 3 `Name`s,
/// `image`, `svg`, and `xml`.
///
/// In all cases, `Name`s are compared case insensitive.
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct Name<'a> {
    // TODO: optimize with an Atom-like thing
    // There a `const` Names, and so it is possible for the static strings
    // to have a different memory address. Additionally, when used in match
    // statements, the strings are compared with a memcmp, possibly even
    // if the address and length are the same.
    //
    // Being an enum with an Atom variant that is a usize (and without a
    // string pointer and boolean) would allow for faster comparisons.
    /// the underlying str slice, which is _required to be lowercase_.
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

    #[inline]
    fn eq_str(&self, s: &str) -> bool {
        //OPTIMIZE: we might parse names into lowercase
        unicase::eq_ascii(self.source, s)
    }

}


impl<'a, 'b> PartialEq<&'b str> for Name<'a> {
    #[inline]
    fn eq(&self, other: & &'b str) -> bool {
        self.eq_str(*other)
    }
}

impl<'a, 'b> PartialEq<Name<'a>> for &'b str {
    #[inline]
    fn eq(&self, other: &Name<'a>) -> bool {
        other.eq_str(*self)
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
    use super::Name;

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
        let param = Name { source: "ABC" };
        let param2 = Name { source: "aBc" };

        // This strange behaviour is a side effect from having to use `derive(PartialEq)`
        // for this type to allow using it in match statements, it is nevertheless acceptable
        // as we do not provide a way to create `Name` outside of this crate and all
        // cases where we create one inside are checked to only use lowercase string
        // as values for source
        assert_ne!(param, param2);
        assert_eq!(param, param2.as_str());
    }
}