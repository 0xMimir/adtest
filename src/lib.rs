#![allow(unused)]
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use syn::{parse::Parse, parse_macro_input, Attribute, Path, Token};

#[proc_macro_attribute]
pub fn adtest(attr: TokenStream, input: TokenStream) -> TokenStream {
    derive_test_function(attr.into(), input).into()
}

fn derive_test_function(attr: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::ItemFn);
    let function_name = input.sig.ident;
    let body = input.block;

    let is_async = input.sig.asyncness.is_some();
    let attrs = parse_macro_input!(attr as AdvanceTestAttrs);
    let needs_async = attrs.needs_async();

    let setup_fn = attrs
        .setup
        .map(InnerFunction::generate_call_function)
        .unwrap_or_default();

    let cleanup_fn = attrs
        .cleanup
        .map(InnerFunction::generate_call_function)
        .unwrap_or_default();

    let (return_type, check_if_error) = match input.sig.output {
        syn::ReturnType::Default => (quote!(Result<(), _>), quote!(test_result.is_err())),
        syn::ReturnType::Type(_, return_type) => (
            quote!(Result<#return_type, _>),
            quote!(
                test_result.is_err() || test_result.ok().map(|r| r.is_err()).unwrap_or_default()
            ),
        ),
    };

    let spawn_code = match is_async {
        true => quote!(tokio::spawn(async move {
            #body
        }).await),
        false => quote!(std::thread::spawn(move || {
            #body
        }).join()),
    };

    let (derive_code, async_sig) = match is_async || needs_async {
        true => (quote!(#[tokio::test]), quote!(async)),
        false => (quote!(#[test]), quote!()),
    };

    quote! {
        #derive_code
        #async_sig fn #function_name(){
            #setup_fn;

            let test_result: #return_type = #spawn_code;

            #cleanup_fn;

            if #check_if_error {
                panic!("Error occurred while running test");
            }
        }
    }
    .into()
}

#[derive(Debug)]
struct InnerFunction {
    name: Path,
    is_async: bool,
}

#[derive(Debug)]
enum Function {
    Setup(InnerFunction),
    Cleanup(InnerFunction),
}

#[derive(Default, Debug)]
struct AdvanceTestAttrs {
    cleanup: Option<InnerFunction>,
    setup: Option<InnerFunction>,
}

impl AdvanceTestAttrs {
    fn needs_async(&self) -> bool {
        self.cleanup.as_ref().is_some_and(|f| f.is_async)
            || self.setup.as_ref().is_some_and(|f| f.is_async)
    }
}

impl Parse for Function {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let func = input.parse::<Ident>()?;

        if func != "setup" && func != "cleanup" {
            return Err(syn::Error::new(
                func.span(),
                "Invalid attribute passed only setup and cleanup are allowed",
            ));
        }

        input.parse::<Token![=]>();

        let is_async = input.peek(Token![async]);
        if is_async {
            input.parse::<Token![async]>()?;
        }

        let function_name = input.parse::<Path>()?;

        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }

        let item = InnerFunction {
            name: function_name,
            is_async,
        };

        match func.to_string().as_str() {
            "setup" => Ok(Self::Setup(item)),
            "cleanup" => Ok(Self::Cleanup(item)),
            _ => panic!("fuck me"),
        }
    }
}

impl Parse for AdvanceTestAttrs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut attr = Self::default();

        while let Ok(enum_function) = input.parse::<Function>() {
            match enum_function {
                Function::Setup(function) => {
                    if attr.setup.is_some() {
                        return Err(syn::Error::new(
                            input.span(),
                            "Setup already defined",
                        ));
                    }
                    attr.setup = Some(function);
                }
                Function::Cleanup(function) => {
                    if attr.setup.is_some() {
                        return Err(syn::Error::new(
                            input.span(),
                            "Cleanup already defined",
                        ));
                    }
                    attr.cleanup = Some(function)
                }
            }
        }

        Ok(attr)
    }
}

impl InnerFunction {
    fn generate_call_function(Self { name, is_async }: Self) -> proc_macro2::TokenStream {
        let should_await = is_async.then_some(quote!(.await));
        quote!(#name()#should_await)
    }
}
