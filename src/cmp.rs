use mime_parse::{Mime, Parser};

pub(crate) fn str_eq(mime: &Mime, s: &str) -> bool {
    if mime.has_params() {
        Parser::can_range()
            .parse(s)
            .map(|other_mime| {
                mime_eq(mime, &other_mime)
            })
            .unwrap_or(false)
    } else {
        mime.as_ref().eq_ignore_ascii_case(s)
    }
}

pub(crate) fn mime_eq(a: &Mime, b: &Mime) -> bool {
    match (a.private_atom(), b.private_atom()) {
        // If either atom is 0, it is "dynamic" and needs to be compared
        // slowly...
        (0, _) | (_, 0) => {
            essence_eq(a, b) && params_eq(a, b)
        },
        (aa, ba) => aa == ba,
    }

}

fn essence_eq(a: &Mime, b: &Mime) -> bool {
    a.essence() == b.essence()
}

fn params_eq(a: &Mime, b: &Mime) -> bool {
    // params size_hint is exact, so if either has more params, they
    // aren't equal.
    if a.params().size_hint() != b.params().size_hint() {
        return false;
    }

    // Order doesn't matter, so we must check simply check that each param
    // exists in both.
    //
    // Most mime types have a small-ish amount of parameters, so
    // scanning the iterators multiple times costs less than creating
    // a temporary hashmap.
    //
    // A simple benchmark suggests a hashmap is faster after about
    // 10 parameters...
    for (name, value) in crate::value::params(a) {
        if crate::value::param(b, name) != Some(value) {
            return false;
        }
    }

    true
}
