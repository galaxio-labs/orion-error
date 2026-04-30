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
