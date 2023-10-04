use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::parse_macro_input;

#[proc_macro_attribute]
pub fn function_name(attr: TokenStream, input: TokenStream) -> TokenStream {
    derive_test_function(attr.into(), input).into()
}

fn derive_test_function(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::ItemFn);
    let function_name = input.sig.ident;
    let body = input.block;

    let mut setup_fn = None;
    let mut cleanup_fn = None;

    for attribute in input.attrs {
        let attribute_name = match attribute.meta.path().get_ident() {
            Some(ident) => ident.to_string(),
            None => continue,
        };

        if attribute_name != "setup" && attribute_name != "cleanup" {
            continue;
        }

        let meta_list = match attribute.meta.require_list() {
            Ok(tokens) => tokens,
            Err(error) => return error.to_compile_error().into(),
        };

        let function = match meta_list.tokens.clone().into_iter().next() {
            Some(function) => function.to_string(),
            None => {
                return syn::Error::new(Span::call_site(), "Function not passed")
                    .into_compile_error()
                    .into()
            }
        };

        let ident = Ident::new(function.to_string().as_str(), Span::call_site());
        let function = quote!(#ident());
        if attribute_name.starts_with("setup") {
            setup_fn = Some(function);
        } else {
            cleanup_fn = Some(function);
        }
    }

    let setup_fn = setup_fn.unwrap_or_default();
    let cleanup_fn = cleanup_fn.unwrap_or_default();

    let (return_type, check_if_error) = match input.sig.output {
        syn::ReturnType::Default => (quote!(Result<(), _>), quote!(test_result.is_err())),
        syn::ReturnType::Type(_, return_type) => (
            quote!(Result<#return_type, _>),
            quote!(
                test_result.is_err() || test_result.ok().map(|r| r.is_err()).unwrap_or_default()
            ),
        ),
    };

    quote! {
        #[test]
        fn #function_name(){
            #setup_fn;

            let test_result: #return_type = std::thread::spawn(move || {
                #body
            }).join();

            #cleanup_fn;

            if #check_if_error {
                panic!("Error incurred while running test");
            }
        }
    }
    .into()
}
