use proc_macro::TokenStream;
use quote::quote;
use syn::ItemFn;

#[proc_macro_attribute]
pub fn main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as ItemFn);
    let fn_ident = input.sig.ident.clone();
    (quote! {
        fn main() -> asyncra::Result<()> {
            #input
            asyncra::wake_runtime(#fn_ident)?;
            Ok(())
        }
    }).into()
}

#[proc_macro_attribute]
pub fn test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as ItemFn);
    let fn_ident = input.sig.ident.clone();
    (quote! {
        #[test]
        fn #fn_ident() -> asyncra::Result<()> {
            #input
            asyncra::wake_runtime(#fn_ident)?;
            Ok(())
        }
    }).into()
}

#[proc_macro_attribute]
pub fn bench(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as ItemFn);
    let fn_ident = input.sig.ident.clone();
    (quote! {
        fn #fn_ident(c: &mut asyncra::Criterion) {
            #input
            c.bench_function(stringify!(#fn_ident), |b| b.iter(|| asyncra::wake_runtime(#fn_ident).unwrap()));
        }
    }).into()
}