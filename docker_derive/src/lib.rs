#![feature(proc_macro_quote)]

use proc_macro::TokenStream;
use quote::quote;
use syn::DeriveInput;

#[proc_macro_derive(Instruction)]
pub fn instr_macro_derive(input: TokenStream) -> TokenStream {
    let s: DeriveInput = syn::parse(input).unwrap();
    impl_instr(&s)
}

fn impl_instr(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let tokens = quote! {
        impl Instruction for #name {}
    };
    tokens.into()
}
