//! Derive macros for `orion-error`.
//!
//! Most downstream crates should depend on `orion-error` and use its default
//! `derive` feature instead of depending on this crate directly.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, spanned::Spanned, Attribute, Data, DeriveInput, Error, Expr, ExprLit,
    ExprPath, Fields, Lit, LitStr, Result, Variant,
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
    let domain_reason = impl_domain_reason(input);

    let mut out = TokenStream2::new();
    let mut errors = Vec::new();
    for result in [display, error_code, identity_provider, domain_reason] {
        match result {
            Ok(tokens) => out.extend(tokens),
            Err(err) => errors.push(err),
        }
    }

    match errors.into_iter().reduce(|mut first, second| {
        first.combine(second);
        first
    }) {
        Some(first) => first.to_compile_error(),
        None => out,
    }
}

fn impl_domain_reason(input: DeriveInput) -> Result<TokenStream2> {
    let ident = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    match input.data {
        Data::Enum(_) | Data::Struct(_) => Ok(quote! {
            impl #impl_generics ::orion_error::DomainReason for #ident #ty_generics #where_clause {}
        }),
        Data::Union(_) => Err(Error::new(
            ident.span(),
            "OrionError can only be derived for enums or structs",
        )),
    }
}

fn impl_display(input: DeriveInput) -> Result<TokenStream2> {
    let ident = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    match input.data {
        Data::Enum(data) => {
            let arms = data
                .variants
                .iter()
                .map(|variant| {
                    let args = OrionAttrs::from_attrs(&variant.attrs)?;
                    if args.transparent {
                        let (pattern, inner) = transparent_variant_pattern(variant)?;
                        Ok(quote! {
                            #pattern => ::std::fmt::Display::fmt(#inner, f)
                        })
                    } else if let Some(message) = args.display_message() {
                        let pattern = variant_pattern(variant);
                        Ok(quote! {
                            #pattern => f.write_str(#message)
                        })
                    } else {
                        Err(Error::new(
                            variant.span(),
                            "missing #[orion_error(message = ...)] or string literal #[orion_error(identity = ...)]",
                        ))
                    }
                })
                .collect::<Result<Vec<_>>>()?;

            Ok(quote! {
                impl #impl_generics ::std::fmt::Display for #ident #ty_generics #where_clause {
                    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                        match self {
                            #(#arms,)*
                        }
                    }
                }
            })
        }
        Data::Struct(data) => {
            let args = OrionAttrs::from_attrs(&input.attrs)?;
            let body = if args.transparent {
                let inner = transparent_struct_binding(&data.fields)?;
                let pattern = struct_pattern(&ident, &data.fields);
                quote! {
                    match self {
                        #pattern => ::std::fmt::Display::fmt(#inner, f),
                    }
                }
            } else if let Some(message) = args.display_message() {
                quote! { f.write_str(#message) }
            } else {
                return Err(Error::new(
                    ident.span(),
                    "missing container #[orion_error(message = ...)] or string literal #[orion_error(identity = ...)]",
                ));
            };

            Ok(quote! {
                impl #impl_generics ::std::fmt::Display for #ident #ty_generics #where_clause {
                    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                        #body
                    }
                }
            })
        }
        Data::Union(_) => Err(Error::new(
            ident.span(),
            "OrionError can only be derived for enums or structs",
        )),
    }
}

#[derive(Clone, Copy)]
enum MissingCode {
    Error,
    Default,
}

fn impl_error_code(input: DeriveInput, missing_code: MissingCode) -> Result<TokenStream2> {
    let ident = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    match input.data {
        Data::Enum(data) => {
            let arms = data
                .variants
                .iter()
                .map(|variant| {
                    let args = OrionAttrs::from_attrs(&variant.attrs)?;
                    if args.transparent {
                        let (pattern, inner) = transparent_variant_pattern(variant)?;
                        Ok(quote! {
                            #pattern => ::orion_error::ErrorCode::error_code(#inner)
                        })
                    } else if let Some(code) = args.code {
                        let pattern = variant_pattern(variant);
                        Ok(quote! {
                            #pattern => #code
                        })
                    } else if matches!(missing_code, MissingCode::Default) {
                        let pattern = variant_pattern(variant);
                        Ok(quote! {
                            #pattern => 500
                        })
                    } else {
                        Err(Error::new(
                            variant.span(),
                            "missing #[orion_error(code = ...)] or #[orion_error(transparent)]",
                        ))
                    }
                })
                .collect::<Result<Vec<_>>>()?;

            Ok(quote! {
                impl #impl_generics ::orion_error::ErrorCode for #ident #ty_generics #where_clause {
                    fn error_code(&self) -> i32 {
                        match self {
                            #(#arms,)*
                        }
                    }
                }
            })
        }
        Data::Struct(data) => {
            let args = OrionAttrs::from_attrs(&input.attrs)?;
            let body = if args.transparent {
                let inner = transparent_struct_binding(&data.fields)?;
                let pattern = struct_pattern(&ident, &data.fields);
                quote! {
                    match self {
                        #pattern => ::orion_error::ErrorCode::error_code(#inner),
                    }
                }
            } else if let Some(code) = args.code {
                quote! { #code }
            } else if matches!(missing_code, MissingCode::Default) {
                quote! { 500 }
            } else {
                return Err(Error::new(
                    ident.span(),
                    "missing container #[orion_error(code = ...)] or #[orion_error(transparent)]",
                ));
            };

            Ok(quote! {
                impl #impl_generics ::orion_error::ErrorCode for #ident #ty_generics #where_clause {
                    fn error_code(&self) -> i32 {
                        #body
                    }
                }
            })
        }
        Data::Union(_) => Err(Error::new(
            ident.span(),
            "ErrorCode can only be derived for enums or structs",
        )),
    }
}

fn impl_error_identity_provider(input: DeriveInput) -> Result<TokenStream2> {
    let ident = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    match input.data {
        Data::Enum(data) => {
            let stable_arms = data
                .variants
                .iter()
                .map(|variant| {
                    let args = OrionAttrs::from_attrs(&variant.attrs)?;
                    if args.transparent {
                        let (pattern, inner) = transparent_variant_pattern(variant)?;
                        Ok(quote! {
                            #pattern => ::orion_error::ErrorIdentityProvider::stable_code(#inner)
                        })
                    } else if let Some(identity) = args.identity {
                        let pattern = variant_pattern(variant);
                        Ok(quote! {
                            #pattern => #identity
                        })
                    } else {
                        Err(Error::new(
                            variant.span(),
                            "missing #[orion_error(identity = ...)] or #[orion_error(transparent)]",
                        ))
                    }
                })
                .collect::<Result<Vec<_>>>()?;

            let category_arms = data
                .variants
                .iter()
                .map(|variant| {
                    let args = OrionAttrs::from_attrs(&variant.attrs)?;
                    if args.transparent {
                        let (pattern, inner) = transparent_variant_pattern(variant)?;
                        Ok(quote! {
                            #pattern => ::orion_error::ErrorIdentityProvider::error_category(#inner)
                        })
                    } else if let Some(category) = args.error_category()? {
                        let pattern = variant_pattern(variant);
                        Ok(quote! {
                            #pattern => #category
                        })
                    } else {
                        Err(Error::new(
                            variant.span(),
                            "missing #[orion_error(category = ...)] or category-prefixed string literal #[orion_error(identity = ...)]",
                        ))
                    }
                })
                .collect::<Result<Vec<_>>>()?;

            Ok(quote! {
                impl #impl_generics ::orion_error::ErrorIdentityProvider for #ident #ty_generics #where_clause {
                    fn stable_code(&self) -> &'static str {
                        match self {
                            #(#stable_arms,)*
                        }
                    }

                    fn error_category(&self) -> ::orion_error::ErrorCategory {
                        match self {
                            #(#category_arms,)*
                        }
                    }
                }
            })
        }
        Data::Struct(data) => {
            let args = OrionAttrs::from_attrs(&input.attrs)?;
            let (stable_body, category_body) = if args.transparent {
                let inner = transparent_struct_binding(&data.fields)?;
                let pattern = struct_pattern(&ident, &data.fields);
                (
                    quote! {
                        match self {
                            #pattern => ::orion_error::ErrorIdentityProvider::stable_code(#inner),
                        }
                    },
                    quote! {
                        match self {
                            #pattern => ::orion_error::ErrorIdentityProvider::error_category(#inner),
                        }
                    },
                )
            } else {
                let identity = args.identity.clone().ok_or_else(|| {
                    Error::new(
                        ident.span(),
                        "missing container #[orion_error(identity = ...)]",
                    )
                })?;
                let category = args.error_category()?.ok_or_else(|| {
                    Error::new(
                        ident.span(),
                        "missing container #[orion_error(category = ...)] or category-prefixed string literal #[orion_error(identity = ...)]",
                    )
                })?;
                (quote! { #identity }, quote! { #category })
            };

            Ok(quote! {
                impl #impl_generics ::orion_error::ErrorIdentityProvider for #ident #ty_generics #where_clause {
                    fn stable_code(&self) -> &'static str {
                        #stable_body
                    }

                    fn error_category(&self) -> ::orion_error::ErrorCategory {
                        #category_body
                    }
                }
            })
        }
        Data::Union(_) => Err(Error::new(
            ident.span(),
            "ErrorIdentityProvider can only be derived for enums or structs",
        )),
    }
}

#[derive(Default)]
struct OrionAttrs {
    message: Option<Expr>,
    code: Option<Expr>,
    identity: Option<Expr>,
    category: Option<TokenStream2>,
    transparent: bool,
}

impl OrionAttrs {
    fn from_attrs(attrs: &[Attribute]) -> Result<Self> {
        let mut out = Self::default();
        for attr in attrs {
            if !attr.path().is_ident("orion_error") {
                continue;
            }

            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("transparent") {
                    out.transparent = true;
                    return Ok(());
                }

                if meta.path.is_ident("code") {
                    out.code = Some(meta.value()?.parse()?);
                    return Ok(());
                }

                if meta.path.is_ident("message") {
                    out.message = Some(meta.value()?.parse()?);
                    return Ok(());
                }

                if meta.path.is_ident("identity") {
                    out.identity = Some(meta.value()?.parse()?);
                    return Ok(());
                }

                if meta.path.is_ident("category") {
                    let expr: Expr = meta.value()?.parse()?;
                    out.category = Some(category_expr(expr)?);
                    return Ok(());
                }

                Err(meta.error("unsupported orion_error attribute"))
            })?;
        }
        Ok(out)
    }

    fn display_message(&self) -> Option<LitStr> {
        self.message
            .as_ref()
            .and_then(expr_lit_str)
            .cloned()
            .or_else(|| {
                self.identity
                    .as_ref()
                    .and_then(expr_lit_str)
                    .map(message_from_identity)
            })
    }

    fn error_category(&self) -> Result<Option<TokenStream2>> {
        if let Some(category) = self.category.clone() {
            return Ok(Some(category));
        }

        let Some(identity) = self.identity.as_ref().and_then(expr_lit_str) else {
            return Ok(None);
        };

        identity_category(identity).transpose()
    }
}

fn expr_lit_str(expr: &Expr) -> Option<&LitStr> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Str(lit), ..
        }) => Some(lit),
        _ => None,
    }
}

fn message_from_identity(identity: &LitStr) -> LitStr {
    let message = identity
        .value()
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .replace('_', " ");
    LitStr::new(&message, identity.span())
}

fn identity_category(identity: &LitStr) -> Option<Result<TokenStream2>> {
    let value = identity.value();
    let prefix = value.split('.').next().unwrap_or_default();
    match prefix {
        "conf" => Some(Ok(quote! { ::orion_error::ErrorCategory::Conf })),
        "biz" => Some(Ok(quote! { ::orion_error::ErrorCategory::Biz })),
        "logic" => Some(Ok(quote! { ::orion_error::ErrorCategory::Logic })),
        "sys" => Some(Ok(quote! { ::orion_error::ErrorCategory::Sys })),
        value => Some(Err(Error::new(
            identity.span(),
            format!(
                "unknown identity category prefix `{value}`; expected one of: conf, biz, logic, sys"
            ),
        ))),
    }
}

fn category_expr(expr: Expr) -> Result<TokenStream2> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Str(lit), ..
        }) => match lit.value().as_str() {
            "conf" => Ok(quote! { ::orion_error::ErrorCategory::Conf }),
            "biz" => Ok(quote! { ::orion_error::ErrorCategory::Biz }),
            "logic" => Ok(quote! { ::orion_error::ErrorCategory::Logic }),
            "sys" => Ok(quote! { ::orion_error::ErrorCategory::Sys }),
            value => Err(Error::new(
                lit.span(),
                format!("unknown error category `{value}`; expected one of: conf, biz, logic, sys"),
            )),
        },
        Expr::Path(ExprPath { path, .. })
            if path.leading_colon.is_none() && path.segments.len() == 1 =>
        {
            let ident = &path.segments[0].ident;
            match ident.to_string().as_str() {
                "Conf" => Ok(quote! { ::orion_error::ErrorCategory::Conf }),
                "Biz" => Ok(quote! { ::orion_error::ErrorCategory::Biz }),
                "Logic" => Ok(quote! { ::orion_error::ErrorCategory::Logic }),
                "Sys" => Ok(quote! { ::orion_error::ErrorCategory::Sys }),
                _ => Ok(path.to_token_stream()),
            }
        }
        other => Ok(other.to_token_stream()),
    }
}

fn variant_pattern(variant: &Variant) -> TokenStream2 {
    let ident = &variant.ident;
    match &variant.fields {
        Fields::Unit => quote! { Self::#ident },
        Fields::Unnamed(_) => quote! { Self::#ident(..) },
        Fields::Named(_) => quote! { Self::#ident { .. } },
    }
}

fn transparent_variant_pattern(variant: &Variant) -> Result<(TokenStream2, TokenStream2)> {
    let ident = &variant.ident;
    match &variant.fields {
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            Ok((quote! { Self::#ident(__inner) }, quote! { __inner }))
        }
        Fields::Named(fields) if fields.named.len() == 1 => {
            let field = fields
                .named
                .iter()
                .next()
                .and_then(|field| field.ident.as_ref())
                .unwrap();
            Ok((quote! { Self::#ident { #field } }, quote! { #field }))
        }
        _ => Err(Error::new(
            variant.span(),
            "#[orion_error(transparent)] requires exactly one field",
        )),
    }
}

fn transparent_struct_binding(fields: &Fields) -> Result<TokenStream2> {
    match fields {
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => Ok(quote! { __inner }),
        Fields::Named(fields) if fields.named.len() == 1 => {
            let field = fields
                .named
                .iter()
                .next()
                .and_then(|field| field.ident.as_ref())
                .unwrap();
            Ok(quote! { #field })
        }
        _ => Err(Error::new(
            fields.span(),
            "#[orion_error(transparent)] requires exactly one field",
        )),
    }
}

fn struct_pattern(ident: &syn::Ident, fields: &Fields) -> TokenStream2 {
    match fields {
        Fields::Unit => quote! { #ident },
        Fields::Unnamed(_) => quote! { #ident(__inner) },
        Fields::Named(fields) => {
            let field = fields
                .named
                .iter()
                .next()
                .and_then(|field| field.ident.as_ref());
            quote! { #ident { #field } }
        }
    }
}
