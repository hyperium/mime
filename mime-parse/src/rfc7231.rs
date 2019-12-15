use crate::{
    as_u16,
    constants,
    Atoms,
    Byte,
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
                byte: Byte(byte),
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
            Some((i, b' ')) if i > start => {
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
                        byte: Byte(byte),
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
                byte: Byte(byte),
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
            debug_assert!({
                let b = s.as_bytes()[start];
                b == b';' || b == b' '
            });
            Atoms::intern(&s[..start], slash, InternParams::None)
        },
        ParamSource::Utf8(params_start) => Atoms::intern(s, slash, InternParams::Utf8(params_start as usize)),
        ParamSource::One(params_start, a) => Source::Dynamic(lower_ascii_with_params(s, params_start as usize, &[a])),
        ParamSource::Two(params_start, a, b) => Source::Dynamic(lower_ascii_with_params(s, params_start as usize, &[a, b])),
        ParamSource::Custom(params_start, ref indices) => Source::Dynamic(lower_ascii_with_params(s, params_start as usize, indices)),
    };

    Ok(Mime {
        source,
        slash,
        plus,
        params,
    })
}


fn params_from_str(s: &str, iter: &mut impl Iterator<Item=(usize, u8)>, mut start: usize) -> Result<ParamSource, ParseError> {
    let params_start = as_u16(start);
    start += 1;
    let mut params = ParamSource::None;
    'params: while start < s.len() {
        let name;
        // name
        'name: loop {
            match iter.next() {
                // OWS
                Some((i, b' ')) if i == start => {
                    start = i + 1;
                    continue 'params;
                },
                // empty param
                Some((i, b';')) if i == start => {
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
                    byte: Byte(byte),
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
                            byte: Byte(byte),
                        }),
                        None => return Err(ParseError::MissingQuote),
                    }

                } else {
                    match iter.next() {
                        Some((i, b'"')) if i > start => {
                            value = (as_u16(start), as_u16(i + 1));
                            start = i + 1;
                            break 'value;
                        },
                        Some((_, b'\\')) => is_quoted_pair = true,
                        Some((_, c)) if is_restricted_quoted_char(c) => (),
                        None => return Err(ParseError::MissingQuote),
                        Some((pos, byte)) => return Err(ParseError::InvalidToken {
                            pos: pos,
                            byte: Byte(byte),
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
                    Some((i, b' ')) |
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
                        byte: Byte(byte),
                    }),
                }
            }
        }

        match params {
            ParamSource::Utf8(i) => {
                let i = i + 2;
                let charset = (i, "charset".len() as u16 + i);
                let utf8 = (charset.1 + 1, charset.1 + "utf-8".len() as u16 + 1);
                params = ParamSource::Two(params_start, (charset, utf8), (name, value));
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
                if params_start + 2 == name.0 &&
                    "charset".eq_ignore_ascii_case(&s[range(name)]) &&
                    "utf-8".eq_ignore_ascii_case(&s[range(value)]) {
                    params = ParamSource::Utf8(params_start);
                    continue 'params;
                }
                params = ParamSource::One(params_start, (name, value));
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

#[cfg(test)]
mod tests {
    fn parse(src: impl super::Parse) -> Result<super::Mime, super::ParseError> {
        super::Parser::can_range().parse(src)
    }

    #[test]
    fn test_lookup_tables() {
        for (i, &valid) in super::TOKEN_MAP.iter().enumerate() {
            let i = i as u8;
            let should = match i {
                b'a'..=b'z' |
                b'A'..=b'Z' |
                b'0'..=b'9' |
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

    #[test]
    fn text_plain() {
        let mime = parse("text/plain").unwrap();
        assert_eq!(mime.type_(), "text");
        assert_eq!(mime.subtype(), "plain");
        assert!(!mime.has_params());
        assert_eq!(mime.as_ref(), "text/plain");
    }

    #[test]
    fn text_plain_uppercase() {
        let mime = parse("TEXT/PLAIN").unwrap();
        assert_eq!(mime.type_(), "text");
        assert_eq!(mime.subtype(), "plain");
        assert!(!mime.has_params());
        assert_eq!(mime.as_ref(), "text/plain");
    }

    #[test]
    fn text_plain_charset_utf8() {
        let mime = parse("text/plain; charset=utf-8").unwrap();
        assert_eq!(mime.type_(), "text");
        assert_eq!(mime.subtype(), "plain");
        assert_eq!(mime.param("charset"), Some("utf-8"));
        assert_eq!(mime.as_ref(), "text/plain; charset=utf-8");
    }

    #[test]
    fn text_plain_charset_utf8_uppercase() {
        let mime = parse("TEXT/PLAIN; CHARSET=UTF-8").unwrap();
        assert_eq!(mime.type_(), "text");
        assert_eq!(mime.subtype(), "plain");
        assert_eq!(mime.param("charset"), Some("utf-8"));
        assert_eq!(mime.as_ref(), "text/plain; charset=utf-8");
    }

    #[test]
    fn text_plain_charset_utf8_quoted() {
        let mime = parse("text/plain; charset=\"utf-8\"").unwrap();
        assert_eq!(mime.type_(), "text");
        assert_eq!(mime.subtype(), "plain");
        assert_eq!(mime.param("charset"), Some("\"utf-8\""));
        assert_eq!(mime.as_ref(), "text/plain; charset=\"utf-8\"");
    }

    #[test]
    fn text_plain_charset_utf8_extra() {
        let mime = parse("text/plain; charset=utf-8; foo=bar").unwrap();
        assert_eq!(mime.type_(), "text");
        assert_eq!(mime.subtype(), "plain");
        assert_eq!(mime.param("charset"), Some("utf-8"));
        assert_eq!(mime.param("foo"), Some("bar"));
        assert_eq!(mime.as_ref(), "text/plain; charset=utf-8; foo=bar");
    }

    #[test]
    fn text_plain_charset_utf8_extra_uppercase() {
        let mime = parse("TEXT/PLAIN; CHARSET=UTF-8; FOO=BAR").unwrap();
        assert_eq!(mime.type_(), "text");
        assert_eq!(mime.subtype(), "plain");
        assert_eq!(mime.param("charset"), Some("utf-8"));
        assert_eq!(mime.param("foo"), Some("BAR"));
        assert_eq!(mime.as_ref(), "text/plain; charset=utf-8; foo=BAR");
    }

    #[test]
    fn charset_utf8_extra_spaces() {
        let mime = parse("text/plain  ;  charset=utf-8  ;  foo=bar").unwrap();
        assert_eq!(mime.type_(), "text");
        assert_eq!(mime.subtype(), "plain");
        assert_eq!(mime.param("charset"), Some("utf-8"));
        assert_eq!(mime.param("foo"), Some("bar"));
        assert_eq!(mime.as_ref(), "text/plain  ;  charset=utf-8  ;  foo=bar");
    }

    #[test]
    fn subtype_space_before_params() {
        let mime = parse("text/plain ; charset=utf-8").unwrap();
        assert_eq!(mime.type_(), "text");
        assert_eq!(mime.subtype(), "plain");
        assert_eq!(mime.param("charset"), Some("utf-8"));
    }

    #[test]
    fn params_space_before_semi() {
        let mime = parse("text/plain; charset=utf-8 ; foo=bar").unwrap();
        assert_eq!(mime.type_(), "text");
        assert_eq!(mime.subtype(), "plain");
        assert_eq!(mime.param("charset"), Some("utf-8"));
    }

    #[test]
    fn param_value_empty_quotes() {
        let mime = parse("audio/wave; codecs=\"\"").unwrap();
        assert_eq!(mime.as_ref(), "audio/wave; codecs=\"\"");
    }

    #[test]
    fn semi_colon_but_empty_params() {
        static CASES: &'static [&'static str] = &[
            "text/event-stream;",
            "text/event-stream; ",
            "text/event-stream;       ",
            "text/event-stream ; ",
        ];

        for &case in CASES {
            let mime = parse(case).expect(case);
            assert_eq!(mime.type_(), "text", "case = {:?}", case);
            assert_eq!(mime.subtype(), "event-stream", "case = {:?}", case);
            assert!(!mime.has_params(), "case = {:?}", case);
            assert_eq!(mime.as_ref(), "text/event-stream", "case = {:?}", case);
        }
    }

    // parse errors

    #[test]
    fn error_type_spaces() {
        parse("te xt/plain").unwrap_err();
    }


    #[test]
    fn error_type_lf() {
        parse("te\nxt/plain").unwrap_err();
    }

    #[test]
    fn error_type_cr() {
        parse("te\rxt/plain").unwrap_err();
    }

    #[test]
    fn error_subtype_spaces() {
        parse("text/plai n").unwrap_err();
    }

    #[test]
    fn error_subtype_crlf() {
        parse("text/\r\nplain").unwrap_err();
    }

    #[test]
    fn error_param_name_crlf() {
        parse("text/plain;\r\ncharset=utf-8").unwrap_err();
    }

    #[test]
    fn error_param_value_quoted_crlf() {
        parse("text/plain;charset=\"\r\nutf-8\"").unwrap_err();
    }

    #[test]
    fn error_param_space_before_equals() {
        parse("text/plain; charset =utf-8").unwrap_err();
    }

    #[test]
    fn error_param_space_after_equals() {
        parse("text/plain; charset= utf-8").unwrap_err();
    }
}
