//! Derive macros for `orion-error`.
//!
//! Most downstream crates should depend on `orion-error` and use its default
//! `derive` feature instead of depending on this crate directly.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, spanned::Spanned, Attribute, Data, DeriveInput, Error, Expr, ExprLit,
    ExprPath, Fields, Ident, Lit, LitStr, Result, Variant,
};

#[proc_macro_derive(ErrorCode, attributes(orion_error))]
pub fn derive_error_code(input: TokenStream) -> TokenStream {
    expand_error_code(parse_macro_input!(input as DeriveInput)).into()
}

#[proc_macro_derive(ErrorIdentityProvider, attributes(orion_error))]
pub fn derive_error_identity_provider(input: TokenStream) -> TokenStream {
    expand_error_identity_provider(parse_macro_input!(input as DeriveInput)).into()
}

#[proc_macro_derive(OrionError, attributes(orion_error))]
pub fn derive_orion_error(input: TokenStream) -> TokenStream {
    expand_orion_error(parse_macro_input!(input as DeriveInput)).into()
}

fn expand_error_code(input: DeriveInput) -> TokenStream2 {
    match impl_error_code(input, MissingCode::Error) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error(),
    }
}

fn expand_error_identity_provider(input: DeriveInput) -> TokenStream2 {
    match impl_error_identity_provider(input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error(),
    }
}

fn expand_orion_error(input: DeriveInput) -> TokenStream2 {
    let display = impl_display(input.clone());
    let error_code = impl_error_code(input.clone(), MissingCode::Default);
    let identity_provider = impl_error_identity_provider(input.clone());
    let domain_reason = impl_domain_reason(input.clone());

    // Generate UnifiedReason delegate constructors if a transparent variant exists
    let uvs_ctors = impl_uvs_constructors(&input)
        .ok()
        .flatten()
        .unwrap_or_default();

    let mut out = TokenStream2::new();
    let mut errors = Vec::new();
    for result in [display, error_code, identity_provider, domain_reason] {
        match result {
            Ok(tokens) => out.extend(tokens),
            Err(err) => errors.push(err),
        }
    }
    out.extend(uvs_ctors);

    match errors.into_iter().reduce(|mut first, second| {
        first.combine(second);
        first
    }) {
        Some(first) => first.to_compile_error(),
        None => out,
    }
}

include!("domain.rs");
include!("display.rs");
include!("error_code.rs");
include!("identity.rs");
include!("attrs.rs");
include!("patterns.rs");
include!("constructors.rs");
