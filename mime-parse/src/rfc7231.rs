use crate::{
    as_u16,
    constants,
    Atoms,
    InternParams,
    lower_ascii_with_params,
    Mime,
    Parse,
    Parser,
    ParseError,
    ParamSource,
    range,
    Source,
};

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

// However, [HTTP](https://tools.ietf.org/html/rfc7231#section-3.1.1.1):
//
// >     media-type = type "/" subtype *( OWS ";" OWS parameter )
// >     type       = token
// >     subtype    = token
// >     parameter  = token "=" ( token / quoted-string )
//
// Where token is defined as:
//
// >     token = 1*tchar
// >     tchar = "!" / "#" / "$" / "%" / "&" / "'" / "*" / "+" / "-" / "." /
// >        "^" / "_" / "`" / "|" / "~" / DIGIT / ALPHA
//
// So, clearly, ¯\_(Ä_/¯

pub(crate) fn parse(opts: &Parser, src: impl Parse) -> Result<Mime, ParseError> {
    let s = src.as_str();
    if s.len() > std::u16::MAX as usize {
        return Err(ParseError::TooLong);
    }

    if s == "*/*" {
        return if opts.can_range {
            Ok(constants::STAR_STAR)
        } else {
            Err(ParseError::InvalidRange)
        };
    }

    let mut iter = s.bytes().enumerate();
    // toplevel
    let mut start;
    let slash;
    loop {
        match iter.next() {
            Some((_, c)) if is_token(c) => (),
            Some((i, b'/')) if i > 0 => {
                slash = as_u16(i);
                start = i + 1;
                break;
            },
            None => return Err(ParseError::MissingSlash), // EOF and no toplevel is no Mime
            Some((pos, byte)) => return Err(ParseError::InvalidToken {
                pos: pos,
                byte: byte,
            }),
        };
    }

    // sublevel
    let mut plus = None;
    loop {
        match iter.next() {
            Some((i, b'+')) if i > start => {
                plus = Some(as_u16(i));
            },
            Some((i, b';')) if i > start => {
                start = i;
                break;
            },

            Some((i, b'*')) if i == start && opts.can_range => {
                // sublevel star can only be the first character, and the next
                // must either be the end, or `;`
                match iter.next() {
                    Some((i, b';')) => {
                        start = i;
                        break;
                    },
                    None => return Ok(Mime {
                        source: Atoms::intern(s, slash, InternParams::None),
                        slash,
                        plus,
                        params: ParamSource::None,
                    }),
                    Some((pos, byte)) => return Err(ParseError::InvalidToken {
                        pos,
                        byte,
                    }),
                }
            },

            Some((_, c)) if is_token(c) => (),
            None => {
                return Ok(Mime {
                    source: Atoms::intern(s, slash, InternParams::None),
                    slash,
                    plus,
                    params: ParamSource::None,
                });
            },
            Some((pos, byte)) => return Err(ParseError::InvalidToken {
                pos: pos,
                byte: byte,
            })
        };
    }

    // params
    let params = params_from_str(s, &mut iter, start)?;

    let source = match params {
        ParamSource::None => {
            // Getting here means there *was* a `;`, but then no parameters
            // after it... So let's just chop off the empty param list.
            debug_assert_ne!(s.len(), start);
            debug_assert_eq!(s.as_bytes()[start], b';');
            Atoms::intern(&s[..start], slash, InternParams::None)
        },
        ParamSource::Utf8(semicolon) => Atoms::intern(s, slash, InternParams::Utf8(semicolon as usize)),
        ParamSource::One(semicolon, a) => Source::Dynamic(lower_ascii_with_params(s, semicolon as usize, &[a])),
        ParamSource::Two(semicolon, a, b) => Source::Dynamic(lower_ascii_with_params(s, semicolon as usize, &[a, b])),
        ParamSource::Custom(semicolon, ref indices) => Source::Dynamic(lower_ascii_with_params(s, semicolon as usize, indices)),
    };

    Ok(Mime {
        source,
        slash,
        plus,
        params,
    })
}


fn params_from_str(s: &str, iter: &mut impl Iterator<Item=(usize, u8)>, mut start: usize) -> Result<ParamSource, ParseError> {
    let semicolon = as_u16(start);
    start += 1;
    let mut params = ParamSource::None;
    'params: while start < s.len() {
        let name;
        // name
        'name: loop {
            match iter.next() {
                Some((i, b' ')) if i == start => {
                    start = i + 1;
                    continue 'params;
                },
                Some((_, c)) if is_token(c) => (),
                Some((i, b'=')) if i > start => {
                    name = (as_u16(start), as_u16(i));
                    start = i + 1;
                    break 'name;
                },
                None => return Err(ParseError::MissingEqual),
                Some((pos, byte)) => return Err(ParseError::InvalidToken {
                    pos: pos,
                    byte: byte,
                }),
            }
        }

        let value;
        // values must be restrict-name-char or "anything goes"
        let mut is_quoted = false;
        let mut is_quoted_pair = false;

        'value: loop {
            if is_quoted {
                if is_quoted_pair {
                    is_quoted_pair = false;
                    match iter.next() {
                        Some((_, ch)) if is_restricted_quoted_char(ch) => (),
                        Some((pos, byte)) => return Err(ParseError::InvalidToken {
                            pos: pos,
                            byte: byte,
                        }),
                        None => return Err(ParseError::MissingQuote),
                    }

                } else {
                    match iter.next() {
                        Some((i, b'"')) if i > start => {
                            value = (as_u16(start), as_u16(i + 1));
                            break 'value;
                        },
                        Some((_, b'\\')) => is_quoted_pair = true,
                        Some((_, c)) if is_restricted_quoted_char(c) => (),
                        None => return Err(ParseError::MissingQuote),
                        Some((pos, byte)) => return Err(ParseError::InvalidToken {
                            pos: pos,
                            byte: byte,
                        }),
                    }
                }
            } else {
                match iter.next() {
                    Some((i, b'"')) if i == start => {
                        is_quoted = true;
                        start = i;
                    },
                    Some((_, c)) if is_token(c) => (),
                    Some((i, b';')) if i > start => {
                        value = (as_u16(start), as_u16(i));
                        start = i + 1;
                        break 'value;
                    }
                    None => {
                        value = (as_u16(start), as_u16(s.len()));
                        start = s.len();
                        break 'value;
                    },

                    Some((pos, byte)) => return Err(ParseError::InvalidToken {
                        pos: pos,
                        byte: byte,
                    }),
                }
            }
        }

        if is_quoted {
            'ws: loop {
                match iter.next() {
                    Some((i, b';')) => {
                        // next param
                        start = i + 1;
                        break 'ws;
                    },
                    Some((_, b' ')) => {
                        // skip whitespace
                    },
                    None => {
                        // eof
                        start = s.len();
                        break 'ws;
                    },
                    Some((pos, byte)) => return Err(ParseError::InvalidToken {
                        pos: pos,
                        byte: byte,
                    }),
                }
            }
        }

        match params {
            ParamSource::Utf8(i) => {
                let i = i + 2;
                let charset = (i, "charset".len() as u16 + i);
                let utf8 = (charset.1 + 1, charset.1 + "utf-8".len() as u16 + 1);
                params = ParamSource::Two(semicolon, (charset, utf8), (name, value));
            },
            ParamSource::One(sc, a) => {
                params = ParamSource::Two(sc, a, (name, value));
            },
            ParamSource::Two(sc, a, b) => {
                params = ParamSource::Custom(sc, vec![a, b, (name, value)]);
            },
            ParamSource::Custom(_, ref mut vec) => {
                vec.push((name, value));
            },
            ParamSource::None => {
                if semicolon + 2 == name.0 &&
                    "charset".eq_ignore_ascii_case(&s[range(name)]) &&
                    "utf-8".eq_ignore_ascii_case(&s[range(value)]) {
                    params = ParamSource::Utf8(semicolon);
                    continue 'params;
                }
                params = ParamSource::One(semicolon, (name, value));
            },
        }
    }
    Ok(params)
}
macro_rules! byte_map {
    ($($flag:expr,)*) => ([
        $($flag != 0,)*
    ])
}

static TOKEN_MAP: [bool; 256] = byte_map![
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 1, 0, 1, 1, 1, 1, 1, 0, 0, 0, 1, 0, 1, 1, 0,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0,
    0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

fn is_token(c: u8) -> bool {
    TOKEN_MAP[c as usize]
}

fn is_restricted_quoted_char(c: u8) -> bool {
    c == 9 || (c > 31 && c != 127)
}

#[test]
fn test_lookup_tables() {
    for (i, &valid) in TOKEN_MAP.iter().enumerate() {
        let i = i as u8;
        let should = match i {
            b'a'...b'z' |
            b'A'...b'Z' |
            b'0'...b'9' |
            b'!' |
            b'#' |
            b'$' |
            b'%' |
            b'&' |
            b'\'' |
            b'+' |
            b'-' |
            b'.' |
            b'^' |
            b'_' |
            b'`' |
            b'|' |
            b'~' => true,
            _ => false
        };
        assert_eq!(valid, should, "{:?} ({}) should be {}", i as char, i, should);
    }
}
