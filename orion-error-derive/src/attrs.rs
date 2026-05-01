#[derive(Default)]
struct OrionAttrs {
    message: Option<Expr>,
    code: Option<Expr>,
    identity: Option<Expr>,
    category: Option<TokenStream2>,
    transparent: bool,
    upcast_from: Vec<Ident>,
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

                if meta.path.is_ident("upcast_from") {
                    let content;
                    syn::parenthesized!(content in meta.input);
                    let mut idents = Vec::new();
                    while !content.is_empty() {
                        idents.push(content.parse::<Ident>()?);
                        if !content.is_empty() {
                            let _ = content.parse::<Token![,]>();
                        }
                    }
                    out.upcast_from = idents;
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
        "conf" => Some(Ok(quote! { ::orion_error::reason::ErrorCategory::Conf })),
        "biz" => Some(Ok(quote! { ::orion_error::reason::ErrorCategory::Biz })),
        "logic" => Some(Ok(quote! { ::orion_error::reason::ErrorCategory::Logic })),
        "sys" => Some(Ok(quote! { ::orion_error::reason::ErrorCategory::Sys })),
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
            "conf" => Ok(quote! { ::orion_error::reason::ErrorCategory::Conf }),
            "biz" => Ok(quote! { ::orion_error::reason::ErrorCategory::Biz }),
            "logic" => Ok(quote! { ::orion_error::reason::ErrorCategory::Logic }),
            "sys" => Ok(quote! { ::orion_error::reason::ErrorCategory::Sys }),
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
                "Conf" => Ok(quote! { ::orion_error::reason::ErrorCategory::Conf }),
                "Biz" => Ok(quote! { ::orion_error::reason::ErrorCategory::Biz }),
                "Logic" => Ok(quote! { ::orion_error::reason::ErrorCategory::Logic }),
                "Sys" => Ok(quote! { ::orion_error::reason::ErrorCategory::Sys }),
                _ => Ok(path.to_token_stream()),
            }
        }
        other => Ok(other.to_token_stream()),
    }
}
