use std::ascii::AsciiExt;
use std::iter::Enumerate;
use std::str::Bytes;

use super::{Mime, Source, Params, Indexed, CHARSET, UTF_8};

#[derive(Debug)]
pub enum ParseError {
    MissingSlash,
    MissingEqual,
    MissingQuote,
    InvalidToken,
}

pub fn parse(s: &str) -> Result<Mime, ParseError> {
    if s == "*/*" {
        return Ok(::STAR_STAR);
    }

    let mut iter = s.bytes().enumerate();
    // toplevel
    let mut start;
    let slash;
    loop {
        match iter.next() {
            Some((0, c)) if is_restricted_name_first_char(c) => (),
            Some((i, c)) if i > 0 && is_restricted_name_char(c) => (),
            Some((i, b'/')) if i > 0 => {
                slash = i;
                start = i + 1;
                break;
            },
            None => return Err(ParseError::MissingSlash), // EOF and no toplevel is no Mime
            _ => return Err(ParseError::InvalidToken)
        };

    }

    // sublevel
    let mut plus = None;
    let mut sub_star = false;
    loop {
        match iter.next() {
            Some((i, b'*')) if i == start => {
                sub_star = true;
            },
            Some((i, c)) if i == start && is_restricted_name_first_char(c) => (),
            Some((i, b'+')) if i > start => {
                plus = Some(i);
            },
            Some((i, b';')) if i > start => {
                start = i;
                break;
            },
            Some((i, c)) if !sub_star && i > start && is_restricted_name_char(c) => (),
            None => {
                return Ok(Mime {
                    source: Source::Dynamic(s.to_ascii_lowercase()),
                    slash: slash,
                    plus: plus,
                    params: Params::None,
                });
            },
            _ => return Err(ParseError::InvalidToken)
        };
    }

    // params
    let params = try!(params_from_str(s, &mut iter, start));

    let src = match params {
        Params::Utf8(_) |
        Params::None => s.to_ascii_lowercase(),
        Params::Custom(semicolon, ref indices) => lower_ascii_with_params(s, semicolon, indices),
    };

    Ok(Mime {
        source: Source::Dynamic(src),
        slash: slash,
        plus: plus,
        params: params,
    })
}


fn params_from_str(s: &str, iter: &mut Enumerate<Bytes>, mut start: usize) -> Result<Params, ParseError> {
    let semicolon = start;
    start += 1;
    let mut params = Params::None;
    'params: while start < s.len() {
        let name;
        // name
        'name: loop {
            match iter.next() {
                Some((i, b' ')) if i == start => start = i + 1,
                Some((i, c)) if i == start && is_restricted_name_first_char(c) => (),
                Some((i, c)) if i > start && is_restricted_name_char(c) => (),
                Some((i, b'=')) if i > start => {
                    name = Indexed(start, i);
                    start = i + 1;
                    break 'name;
                },
                None => return Err(ParseError::MissingEqual),
                _ => return Err(ParseError::InvalidToken),
            }
        }

        let value;
        // values must be restrict-name-char or "anything goes"
        let mut is_quoted = false;

        'value: loop {
            if is_quoted {
                match iter.next() {
                    Some((i, b'"')) if i > start => {
                        value = Indexed(start, i);
                        start = i + 1;
                        break 'value;
                    },
                    Some((_, c)) if is_restricted_quoted_char(c) => (),
                    None => return Err(ParseError::MissingQuote),
                    _ => return Err(ParseError::InvalidToken),
                }

            } else {
                match iter.next() {
                    Some((i, b'"')) if i == start => {
                        is_quoted = true;
                        start = i + 1;
                    },
                    Some((i, c)) if i == start && is_restricted_name_first_char(c) => (),
                    Some((i, c)) if i > start && is_restricted_name_char(c) => (),
                    Some((i, b';')) if i > start => {
                        value = Indexed(start, i);
                        start = i + 1;
                        break 'value;
                    }
                    None => {
                        value = Indexed(start, s.len());
                        start = s.len();
                        break 'value;
                    },

                    _ => return Err(ParseError::InvalidToken),
                }
            }
        }

        match params {
            Params::Utf8(i) => {
                let i = i + 2;
                let charset = Indexed(i, "charset".len() + i);
                let utf8 = Indexed(charset.1 + 1, charset.1 + "utf-8".len() + 1);
                params = Params::Custom(semicolon, vec![
                    (charset, utf8),
                    (name, value),
                ]);
            },
            Params::Custom(_, ref mut vec) => {
                vec.push((name, value));
            },
            Params::None => {
                if semicolon + 2 == name.0 && CHARSET == &s[name.0..name.1] {
                    if UTF_8 == &s[value.0..value.1] {
                        params = Params::Utf8(semicolon);
                        continue 'params;
                    }
                }
                params = Params::Custom(semicolon, vec![(name, value)]);
            },
        }
    }
    Ok(params)
}

fn lower_ascii_with_params(s: &str, semi: usize, params: &[(Indexed, Indexed)]) -> String {
    let mut owned = s.to_owned();
    owned[..semi].make_ascii_lowercase();

    for &(ref name, ref value) in params {
        owned[name.0..name.1].make_ascii_lowercase();
        // Since we just converted this part of the string to lowercase,
        // we can skip the `Name == &str` unicase check and do a faster
        // memcmp instead.
        if &owned[name.0..name.1] == CHARSET.source {
            owned[value.0..value.1].make_ascii_lowercase();
        }
    }

    owned
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


macro_rules! byte_map {
    ($($flag:expr,)*) => ([
        $($flag != 0,)*
    ])
}


static RESTRICTED_NAME_FIRST: [bool; 256] = byte_map![
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0,
    0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0,
    0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

static RESTRICTED_NAME_CHAR: [bool; 256] = byte_map![
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 1, 0, 1, 1, 0, 1, 0, 0, 0, 0, 1, 0, 1, 1, 0,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0,
    0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1,
    0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

fn is_restricted_name_first_char(c: u8) -> bool {
    RESTRICTED_NAME_FIRST[c as usize]
}

fn is_restricted_name_char(c: u8) -> bool {
    RESTRICTED_NAME_CHAR[c as usize]
}

fn is_restricted_quoted_char(c: u8) -> bool {
    c > 31 && c != 127
}

#[test]
fn test_lookup_tables() {
    for (i, &valid) in RESTRICTED_NAME_FIRST.iter().enumerate() {
        let i = i as u8;
        let should = match i {
            b'a'...b'z' |
            b'A'...b'Z' |
            b'0'...b'9' => true,
            _ => false
        };
        assert_eq!(valid, should, "{:?} ({}) should be {}", i as char, i, should);
    }
    for (i, &valid) in RESTRICTED_NAME_CHAR.iter().enumerate() {
        let i = i as u8;
        let should = match i {
            b'a'...b'z' |
            b'A'...b'Z' |
            b'0'...b'9' |
            b'!' |
            b'#' |
            b'$' |
            b'&' |
            b'-' |
            b'^' |
            b'.' |
            b'+' |
            b'_' => true,
            _ => false
        };
        assert_eq!(valid, should, "{:?} ({}) should be {}", i as char, i, should);
    }
}
