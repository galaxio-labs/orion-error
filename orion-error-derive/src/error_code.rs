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
                            #pattern => ::orion_error::reason::ErrorCode::error_code(#inner)
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
                impl #impl_generics ::orion_error::reason::ErrorCode for #ident #ty_generics #where_clause {
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
                        #pattern => ::orion_error::reason::ErrorCode::error_code(#inner),
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
                impl #impl_generics ::orion_error::reason::ErrorCode for #ident #ty_generics #where_clause {
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
