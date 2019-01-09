extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro_hack::proc_macro_hack;
use proc_macro2::Span;
use quote::quote;

#[proc_macro_hack]
pub fn media_type(tokens: TokenStream) -> TokenStream {
    let lit_str = syn::parse_macro_input!(tokens as syn::LitStr);

    let mime = match parse_mime_lit(&lit_str.value()) {
        Ok(mime) => mime,
        Err(msg) => {
            let err = syn::Error::new(Span::call_site(), msg);
            return err.to_compile_error().into();
        }
    };

    let source = match mime.source {
        mime_parse::Source::Atom(a, s) => quote! {
            $crate::private::Source::Atom(#a, #s)
        },
        mime_parse::Source::Dynamic(s) => quote! {
            // Atom 0 is a dynamic-but-still-static
            $crate::private::Source::Atom(0, #s)
        },
    };
    let slash = mime.slash;
    let plus = match mime.plus {
        Some(i) => quote! { ::std::option::Option::Some(#i) },
        None => quote! { ::std::option::Option::None },
    };
    let params = match mime.params {
        mime_parse::ParamSource::None => quote! { $crate::private::ParamSource::None },
        mime_parse::ParamSource::Utf8(sc) => quote! { $crate::private::ParamSource::Utf8(#sc) },
        mime_parse::ParamSource::One(sc, ((na, nz), (va, vz))) => quote! {
            $crate::private::ParamSource::One(#sc, ((#na, #nz), (#va, #vz)))
        },
        _ => unreachable!("custom params quote"),
    };

    let out = quote! {
        unsafe {
            $crate::MediaType::private_from_proc_macro($crate::private::Mime {
                source: #source,
                slash: #slash,
                plus: #plus,
                params: #params,
            })
        }
    };
    out.into()
}

fn parse_mime_lit(value: &str) -> Result<mime_parse::Mime, String> {
    let mime = mime_parse::parse(
        value,
        mime_parse::CanRange::No,
    );

    match mime {
        Ok(mime) => match mime.params {
            mime_parse::ParamSource::None |
            mime_parse::ParamSource::Utf8(_) => Ok(mime),
            mime_parse::ParamSource::One(..) => Ok(mime),
            _ => Err("multiple parameters not supported yet".into())
        },
        Err(err) => {
            Err(format!("invalid MediaType: {}", err))
        }
    }
}
