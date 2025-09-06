extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = syn::parse_macro_input!(item as syn::ItemFn);

    // Extract the function name and body
    let syn::ItemFn { attrs, vis, sig, block } = input;

    // Generate the output tokens
    let output = quote! {
        #[tokio::main]
        #(#attrs)*
        #vis #sig {
            #block
        }
    };

    // Convert the output back to a TokenStream
    TokenStream::from(output)
}
