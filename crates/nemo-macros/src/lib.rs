//! Derive macros for Nemo components.
//!
//! Provides `NemoComponent` which generates a `new(component: BuiltComponent) -> Self`
//! constructor that extracts properties from a `BuiltComponent`.
//!
//! # Field Attributes
//!
//! - `#[property]` — extract using field name as the property key
//! - `#[property(default = <value>)]` — with a default when the property is absent
//! - `#[property(name = "<key>")]` — use a different property key than the field name
//! - `#[children]` — marks a `Vec<AnyElement>` field; generates a `children()` builder
//! - `#[source]` — stores the full `BuiltComponent` for handler/id access
//!
//! # Supported Property Types
//!
//! `String`, `i64`, `f64`, `bool`, and `Option<T>` variants of each.

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Field, Fields, GenericArgument, Lit, Meta, PathArguments, Type};

#[proc_macro_derive(NemoComponent, attributes(property, children, source))]
pub fn derive_nemo_component(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return syn::Error::new_spanned(
                    &input.ident,
                    "NemoComponent only supports structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(&input.ident, "NemoComponent only supports structs")
                .to_compile_error()
                .into();
        }
    };

    let mut let_bindings = Vec::new();
    let mut field_assigns = Vec::new();
    let mut has_children = false;
    let mut source_field_name = None;

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();

        if has_attr(field, "children") {
            has_children = true;
            field_assigns.push(quote! { #field_name: vec![] });
        } else if has_attr(field, "source") {
            source_field_name = Some(field_name.clone());
        } else if has_attr(field, "property") {
            let (prop_name, default_value) = parse_property_attr(field);
            let prop_key = prop_name.unwrap_or_else(|| field_name.to_string());
            let extraction = generate_extraction(&field.ty, &prop_key, default_value.as_ref());
            let_bindings.push(quote! { let #field_name = #extraction; });
            field_assigns.push(quote! { #field_name });
        } else {
            field_assigns.push(quote! { #field_name: Default::default() });
        }
    }

    // Source field must be assigned last (after property extractions borrow component).
    if let Some(ref name) = source_field_name {
        field_assigns.push(quote! { #name: component });
    }

    let children_method = if has_children {
        quote! {
            pub fn children(mut self, children: Vec<gpui::AnyElement>) -> Self {
                self.children = children;
                self
            }
        }
    } else {
        quote! {}
    };

    let expanded = quote! {
        impl #name {
            pub fn new(component: nemo_layout::BuiltComponent) -> Self {
                #(#let_bindings)*
                Self {
                    #(#field_assigns),*
                }
            }

            #children_method
        }
    };

    TokenStream::from(expanded)
}

fn has_attr(field: &Field, name: &str) -> bool {
    field.attrs.iter().any(|attr| attr.path().is_ident(name))
}

fn parse_property_attr(field: &Field) -> (Option<String>, Option<Lit>) {
    let mut prop_name = None;
    let mut default_value = None;

    for attr in &field.attrs {
        if !attr.path().is_ident("property") {
            continue;
        }

        if matches!(attr.meta, Meta::Path(_)) {
            return (None, None);
        }

        if let Meta::List(_) = &attr.meta {
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("name") {
                    let value = meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    prop_name = Some(s.value());
                } else if meta.path.is_ident("default") {
                    let value = meta.value()?;
                    let lit: Lit = value.parse()?;
                    default_value = Some(lit);
                }
                Ok(())
            });
        }
    }

    (prop_name, default_value)
}

fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        type_path
            .path
            .segments
            .last()
            .map(|seg| seg.ident == "Option")
            .unwrap_or(false)
    } else {
        false
    }
}

fn get_inner_type(ty: &Type) -> &Type {
    if let Type::Path(type_path) = ty {
        let segment = type_path.path.segments.last().unwrap();
        if let PathArguments::AngleBracketed(args) = &segment.arguments {
            if let Some(GenericArgument::Type(inner)) = args.args.first() {
                return inner;
            }
        }
    }
    panic!("Expected Option<T> type");
}

fn get_type_name(ty: &Type) -> String {
    if let Type::Path(type_path) = ty {
        type_path.path.segments.last().unwrap().ident.to_string()
    } else {
        panic!("Unsupported type");
    }
}

fn generate_extraction(
    ty: &Type,
    prop_key: &str,
    default: Option<&Lit>,
) -> proc_macro2::TokenStream {
    let is_optional = is_option_type(ty);
    let inner_type = if is_optional { get_inner_type(ty) } else { ty };
    let type_name = get_type_name(inner_type);

    let accessor = match type_name.as_str() {
        "String" => quote! {
            component.properties.get(#prop_key).map(|v| match v.as_str() {
                Some(s) => s.to_string(),
                None => v.to_string(),
            })
        },
        "i64" => quote! {
            component.properties.get(#prop_key).and_then(|v| v.as_i64())
        },
        "f64" => quote! {
            component.properties.get(#prop_key).and_then(|v| v.as_f64())
        },
        "bool" => quote! {
            component.properties.get(#prop_key).and_then(|v| v.as_bool())
        },
        _ => {
            let msg = format!("Unsupported property type: {}", type_name);
            return quote! { compile_error!(#msg) };
        }
    };

    if is_optional {
        accessor
    } else if let Some(default_lit) = default {
        match type_name.as_str() {
            "String" => quote! { #accessor.unwrap_or_else(|| #default_lit.to_string()) },
            _ => quote! { #accessor.unwrap_or(#default_lit) },
        }
    } else {
        quote! { #accessor.unwrap_or_default() }
    }
}
