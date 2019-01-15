
macro_rules! mimes {
    ($(@ $kind:ident: $($id:ident, $src:expr;)+)+) => (
        pub(super) mod mimes {
            use crate::{MediaType, MediaRange};
        $($(
            mime_constant! {
                $kind, $id, $src
            }
        )+)+
        }


        #[test]
        fn test_mimes_macro_consts() {
            use self::mimes::*;
            $($(
            mime_constant_test! {
                $id, $src
            }
            )+)+


            $($(
            mime_constant_proc_macro_test! {
                @$kind, $id, $src
            }
            )+)+
        }
    )
}

macro_rules! mime_constant {
    ($kind:ident, $id:ident, $src:expr) => (
        mime_constant! {
            @DOC concat!("A `", stringify!($kind), "` representing `\"", $src, "\"`."),
            $kind,
            $id,
            $src
        }
    );
    (@DOC $doc:expr, $kind:ident, $id:ident, $src:expr) => (
        #[doc = $doc]
        pub const $id: $kind = $kind {
            mime: mime_parse::constants::$id,
        };
    )
}

#[cfg(test)]
macro_rules! mime_constant_test {
    ($id:ident, $src:expr) => ({
        let __mime = $id;

        // prevent ranges from being MediaTypes
        __mime.test_assert_asterisks();
    })
}

#[cfg(test)]
macro_rules! mime_constant_proc_macro_test {
    (@MediaType, $id:ident, $src:expr) => (
        // Test proc macro matches constants
        #[cfg(feature = "macro")]
        {
            let constant = $id;
            let macroed = media_type!($src);
            assert_eq!(constant.type_(), macroed.type_());
            assert_eq!(constant.subtype(), macroed.subtype());
            assert_eq!(constant.suffix(), macroed.suffix());
            assert_ne!(macroed.mime.private_atom(), 0);
            assert_eq!(constant.mime.private_atom(), macroed.mime.private_atom());
        }
    );
    (@MediaRange, $id:ident, $src:expr) => ();
}

mimes! {
    @ MediaType:
    TEXT_PLAIN, "text/plain";
    TEXT_PLAIN_UTF_8, "text/plain; charset=utf-8";
    TEXT_HTML, "text/html";
    TEXT_HTML_UTF_8, "text/html; charset=utf-8";
    TEXT_CSS, "text/css";
    TEXT_CSS_UTF_8, "text/css; charset=utf-8";
    TEXT_JAVASCRIPT, "text/javascript";
    TEXT_XML, "text/xml";
    TEXT_EVENT_STREAM, "text/event-stream";
    TEXT_CSV, "text/csv";
    TEXT_CSV_UTF_8, "text/csv; charset=utf-8";
    TEXT_TAB_SEPARATED_VALUES, "text/tab-separated-values";
    TEXT_TAB_SEPARATED_VALUES_UTF_8, "text/tab-separated-values; charset=utf-8";
    TEXT_VCARD, "text/vcard";

    IMAGE_JPEG, "image/jpeg";
    IMAGE_GIF, "image/gif";
    IMAGE_PNG, "image/png";
    IMAGE_BMP, "image/bmp";
    IMAGE_SVG, "image/svg+xml";

    FONT_WOFF, "font/woff";
    FONT_WOFF2, "font/woff2";

    APPLICATION_JSON, "application/json";
    APPLICATION_JAVASCRIPT, "application/javascript";
    APPLICATION_JAVASCRIPT_UTF_8, "application/javascript; charset=utf-8";
    APPLICATION_WWW_FORM_URLENCODED, "application/x-www-form-urlencoded";
    APPLICATION_OCTET_STREAM, "application/octet-stream";
    APPLICATION_MSGPACK, "application/msgpack";
    APPLICATION_PDF, "application/pdf";
    APPLICATION_DNS, "application/dns-message";

    // media-ranges
    @ MediaRange:
    STAR_STAR, "*/*";
    TEXT_STAR, "text/*";
    IMAGE_STAR, "image/*";
    VIDEO_STAR, "video/*";
    AUDIO_STAR, "audio/*";
}

