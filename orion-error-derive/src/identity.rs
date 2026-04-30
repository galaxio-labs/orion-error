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
                            #pattern => ::orion_error::reason::ErrorIdentityProvider::stable_code(#inner)
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
                            #pattern => ::orion_error::reason::ErrorIdentityProvider::error_category(#inner)
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
                impl #impl_generics ::orion_error::reason::ErrorIdentityProvider for #ident #ty_generics #where_clause {
                    fn stable_code(&self) -> &'static str {
                        match self {
                            #(#stable_arms,)*
                        }
                    }

                    fn error_category(&self) -> ::orion_error::reason::ErrorCategory {
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
                            #pattern => ::orion_error::reason::ErrorIdentityProvider::stable_code(#inner),
                        }
                    },
                    quote! {
                        match self {
                            #pattern => ::orion_error::reason::ErrorIdentityProvider::error_category(#inner),
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
                impl #impl_generics ::orion_error::reason::ErrorIdentityProvider for #ident #ty_generics #where_clause {
                    fn stable_code(&self) -> &'static str {
                        #stable_body
                    }

                    fn error_category(&self) -> ::orion_error::reason::ErrorCategory {
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
