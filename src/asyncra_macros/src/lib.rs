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
            asyncra::wake_runtime(#fn_ident, asyncra::event_loop)?;
            Ok(())
        }
    }).into()
}