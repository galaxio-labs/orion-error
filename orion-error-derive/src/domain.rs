fn impl_domain_reason(input: DeriveInput) -> Result<TokenStream2> {
    let ident = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    match input.data {
        Data::Enum(_) | Data::Struct(_) => Ok(quote! {
            impl #impl_generics ::orion_error::reason::DomainReason for #ident #ty_generics #where_clause {}
        }),
        Data::Union(_) => Err(Error::new(
            ident.span(),
            "OrionError can only be derived for enums or structs",
        )),
    }
}
