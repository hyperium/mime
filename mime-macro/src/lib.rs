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

    let source = match mime.private_atom() {
        0 => {
            let s = mime.as_ref();
            // Atom 0 is a dynamic-but-still-static
            quote! {
                $crate::private::Source::Atom(0, #s)
            }
        },
        a => {
            let s = mime.as_ref();
            quote! {
                $crate::private::Source::Atom(#a, #s)
            }
        },
    };
    let slash = mime.private_subtype_offset();
    let plus = match mime.private_suffix_offset() {
        Some(i) => quote! { ::std::option::Option::Some(#i) },
        None => quote! { ::std::option::Option::None },
    };
    let params = match mime.private_params_source() {
        mime_parse::ParamSource::None => quote! { $crate::private::ParamSource::None },
        mime_parse::ParamSource::Utf8(sc) => quote! { $crate::private::ParamSource::Utf8(#sc) },
        mime_parse::ParamSource::One(sc, ((na, nz), (va, vz))) => quote! {
            $crate::private::ParamSource::One(#sc, ((#na, #nz), (#va, #vz)))
        },
        _ => unreachable!("custom params quote"),
    };

    let out = quote! {
        unsafe {
            $crate::MediaType::private_from_proc_macro(
                $crate::private::Mime::private_from_proc_macro(
                    #source,
                    #slash,
                    #plus,
                    #params,
                )
            )
        }
    };
    out.into()
}

fn parse_mime_lit(value: &str) -> Result<mime_parse::Mime, String> {
    let mime = mime_parse::Parser::cannot_range().parse(value);

    match mime {
        Ok(mime) => match mime.private_params_source() {
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
