pub(crate) fn impl_upcast_from(
    ident: &Ident,
    attrs: &OrionAttrs,
) -> Result<Vec<TokenStream2>> {
    let target = ident;

    let mut impls = Vec::with_capacity(attrs.upcast_from.len());
    for source in &attrs.upcast_from {
        impls.push(quote! {
            impl ::std::convert::From<::orion_error::StructError<#source>>
                for ::orion_error::StructError<#target>
            where
                #source: ::orion_error::reason::DomainReason,
                #target: ::orion_error::reason::DomainReason + ::std::convert::From<#source>,
            {
                fn from(
                    other: ::orion_error::StructError<#source>,
                ) -> Self {
                    ::orion_error::convert_error(other)
                }
            }
        });
    }

    Ok(impls)
}
