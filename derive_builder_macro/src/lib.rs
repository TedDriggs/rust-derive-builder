

#![crate_type = "proc-macro"]
#![deny(warnings)]

extern crate proc_macro;
#[macro_use]
extern crate darling;
extern crate syn;
#[macro_use]
extern crate quote;
#[cfg(feature = "logging")]
#[macro_use]
extern crate log;
#[cfg(feature = "logging")]
extern crate env_logger;
extern crate derive_builder_core;

#[cfg(not(feature = "logging"))]
#[macro_use]
mod log_disabled;
mod options;

use proc_macro::TokenStream;
use darling::{FromDeriveInput, FromField};
#[cfg(feature = "logging")]
use std::sync::{Once, ONCE_INIT};

#[cfg(feature = "logging")]
static INIT_LOGGER: Once = ONCE_INIT;

#[doc(hidden)]
#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    #[cfg(feature = "logging")]
    INIT_LOGGER.call_once(|| {
        env_logger::init().unwrap();
    });

    let input = input.to_string();

    let ast = syn::parse_macro_input(&input).expect("Couldn't parse item");

    let result = builder_for_struct(ast).to_string();
    debug!("generated tokens: {}", result);

    result.parse().expect(&format!("Couldn't parse `{}` to tokens", result))
}

fn builder_for_struct(ast: syn::MacroInput) -> quote::Tokens {
    debug!("Deriving Builder for `{}`.", ast.ident);

    let s_level = options::StructOptions::from_derive_input(&ast).unwrap();

    let fields = match ast.body {
        syn::Body::Struct(syn::VariantData::Struct(fields)) => fields,
        _ => panic!("`#[derive(Builder)]` can only be used with braced structs"),
    };

    let mut builder = s_level.as_builder();
    let mut build_fn = s_level.as_build_method();

    for f in fields {
        let mut f_level = options::FieldOptions::from_field(&f).unwrap();
        f_level.with_defaults(&s_level);


        builder.push_field(f_level.as_builder_field());
        builder.push_setter_fn(f_level.as_setter());
        build_fn.push_initializer(f_level.as_initializer());
    }

    builder.doc_comment(format!(include_str!("doc_tpl/builder_struct.md"),
                                struct_name = ast.ident.as_ref()));
    build_fn.doc_comment(format!(include_str!("doc_tpl/builder_method.md"),
                                struct_name = ast.ident.as_ref()));

    builder.push_build_fn(build_fn);

    quote!(#builder)
}
