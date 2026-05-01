// Generation of delegate constructors for transparent UvsReason variants.
// When a domain reason enum has #[orion_error(transparent)] wrapping UvsReason,
// the derive generates all UvsReason constructors as methods on the enum.

const UVS_METHODS: &[&str] = &[
    "core_conf",
    "feature_conf",
    "dynamic_conf",
    "validation_error",
    "business_error",
    "rule_error",
    "not_found_error",
    "permission_error",
    "data_error",
    "system_error",
    "network_error",
    "resource_error",
    "timeout_error",
    "external_error",
    "logic_error",
];

pub(crate) fn impl_uvs_constructors(input: &DeriveInput) -> Result<Option<TokenStream2>> {
    let data = match &input.data {
        Data::Enum(e) => e,
        _ => return Ok(None),
    };

    // Find the transparent variant that wraps UvsReason.
    let uvs_variant = data.variants.iter().find(|variant| {
        let attrs = OrionAttrs::from_attrs(&variant.attrs).ok();
        let is_transparent = attrs.map(|a| a.transparent).unwrap_or(false);
        if !is_transparent {
            return false;
        }
        // Check the inner type name
        match &variant.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let ty_str = quote!(#fields).to_string();
                // Matches "UvsReason" or "::orion_error::reason::UvsReason"
                ty_str.contains("UvsReason")
            }
            _ => false,
        }
    });

    let Some(variant) = uvs_variant else {
        return Ok(None);
    };

    let variant_ident = &variant.ident;
    let enum_ident = &input.ident;

    let methods: Vec<_> = UVS_METHODS
        .iter()
        .map(|method| {
            let method_name = Ident::new(method, variant_ident.span());
            quote! {
                pub fn #method_name() -> Self {
                    Self::#variant_ident(
                        ::orion_error::reason::UvsReason::#method_name()
                    )
                }
            }
        })
        .collect();

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(Some(quote! {
        impl #impl_generics #enum_ident #ty_generics #where_clause {
            #(#methods)*
        }
    }))
}
