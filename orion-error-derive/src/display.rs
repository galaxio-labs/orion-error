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
