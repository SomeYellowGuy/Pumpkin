//! This module provides `derive` proc macros for `Encode` and `Decode`.
//!
//! # Attributes
//!
//! Every attribute specified here is a sub-attribute; the root attribute must be  `#[codec(...)]`.
//!
//! ## Field Attributes
//! - `skip`: Skips serializing the field entirely, and instead uses the value provided by `default`, if any.
//!   In this case, `default` is *optional*, and if it is not specified, this falls back to `Default::default()`.
//! - `name = "x"`: Sets this field to be encoded with the key `"x"`. If not specified, the field's name
//!   defaults to the Rust field's name. This is usually required for tuple fields.
//! - `default` or `default = ...`: Sets a default value for a field. If no value is specified, it defaults to `Default::default()`.
//!   This is used for skipped fields and encoding *defaulted* fields.
//! - `lenient`: Only for encoding `Option`s and defaulted fields. If a value is present and cannot be successfully decoded,
//!   the value is ignored and a `None`/the default value is decoded instead.
//! - `flatten`: Flattens key-value pairs of a field whose type implements `MapEncode`/`MapDecode`.
//!   This cannot be used with `default` and functional field attributes (given later).
//!
//! ### Functional Field Attributes
//! The order of these attributes matters because transformations are applied sequentially. The ones specified first
//! are applied first.
//!
//! - `validate = "func"`: Validates a field's value before encoding and/or after decoding. The `func`'s signature must be `(&T) -> Result<(), S>`,
//!   where `S: Into<String>`. Two common types of `S` are `String` and `&str`.
//!
//! ## Struct/Enum Body Attributes
//! - `tag_key = "x"` on `enum`s: Tells the key for storing the enum's type. This is used to differentiate the variant
//!   during decoding. If omitted, this defaults to `"type"`.
//! - `rename_all = "x"`: Changes the tags of all enum variants, whose tag is not overridden already, to be of a certain case's
//!   version of the variant's name. The valid options are:
//!   - `"UPPERCASE"`
//!   - `"lowercase"`
//!   - `"snake_case"`
//!   - `"PascalCase"`
//!   - `"camelCase"`
//! - `transparent`: Only for structs. If a struct has exactly 1 field, instead of encoding to/decoding a map, the struct
//!   will be represented by how that field's value is represented as well. If this attribute is used, no naming
//!   of the single field is required, and it will be ignored. Obviously, this cannot be used with `flatten`.
//!
//! ## Enum Variant Attributes
//! - `tag = "x"`: Tells the value for storing the enum's type. This is used to differentiate the variant
//!   during decoding. If not specified, defaults to using the `snake_case` version of the variant's name
//!   (or whatever the `rename_all` attribute says)

mod attribute;
mod decode;
mod encode;
mod field;

use crate::attribute::{ParsedAttribute, add_attribute_branch};
use heck::{ToLowerCamelCase, ToPascalCase, ToSnakeCase};
use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro_error2::__export::proc_macro2;
use proc_macro_error2::__export::proc_macro2::{Ident, Span};
use quote::{ToTokens, quote};
use syn::{Attribute, DeriveInput, Error, LitStr, Path, Type, parse_macro_input};

/// Returns the tokens corresponding to the `pumpkin_codecs` crate.
fn crate_token() -> proc_macro2::TokenStream {
    match crate_name("pumpkin-codecs") {
        Ok(FoundCrate::Itself) => quote! { crate },
        Ok(FoundCrate::Name(name)) => Ident::new(&name, Span::call_site()).into_token_stream(),
        Err(_) => Ident::new("pumpkin_codecs", Span::call_site()).into_token_stream(),
    }
}

/// Derives the `Encode` trait for a struct.
///
/// This trait also derives `MapEncode` (except for enums whose variants are all units and unit structs),
/// though this trait may only be useful directly for certain cases,
/// which is then used to derive `Encode`.
///
/// Check the [module's documentation](crate) for every attribute you can use.
#[proc_macro_derive(Encode, attributes(codec))]
pub fn derive_encode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    encode::derive_encode(&crate_token(), &input).unwrap_or_else(|e| e.to_compile_error().into())
}

/// Derives the `Decode` trait for a struct.
///
/// This trait also derives `MapDecode` (except for enums whose variants are all units and unit structs),
/// though this trait may only be useful directly for certain cases,
/// which is then used to derive `Decode`.
///
/// Check the [module's documentation](crate) for every attribute you can use.
#[proc_macro_derive(Decode, attributes(codec))]
pub fn derive_decode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    decode::derive_decode(&crate_token(), &input).unwrap_or_else(|e| e.to_compile_error().into())
}

#[derive(Debug, Copy, Clone)]
enum RenameAllOption {
    None,
    UpperCase,
    LowerCase,
    SnakeCase,
    PascalCase,
    CamelCase,
}

impl RenameAllOption {
    pub fn apply(self, name: &str) -> String {
        match self {
            // Default to snake case.
            Self::None => Self::SnakeCase.apply(name),

            Self::UpperCase => name.to_uppercase(),
            Self::LowerCase => name.to_lowercase(),
            Self::SnakeCase => name.to_snake_case(),
            Self::PascalCase => name.to_pascal_case(),
            Self::CamelCase => name.to_lower_camel_case(),
        }
    }
}

impl TryFrom<&LitStr> for RenameAllOption {
    type Error = Error;

    fn try_from(value: &LitStr) -> Result<Self, Self::Error> {
        match value.value().as_str() {
            "lowercase" => Ok(Self::LowerCase),
            "UPPERCASE" => Ok(Self::UpperCase),
            "snake_case" => Ok(Self::SnakeCase),
            "PascalCase" => Ok(Self::PascalCase),
            "camelCase" => Ok(Self::CamelCase),
            s => Err(Error::new(
                value.span(),
                format!("Invalid `rename_all` option: {s}"),
            )),
        }
    }
}

struct EnumDispatchData {
    tag_key: String,
    rename_all: RenameAllOption,
}

fn duplicate_attribute_error(ident: &Ident) -> Error {
    Error::new_spanned(
        ident,
        format!("The `{ident}` attribute was already defined"),
    )
}

struct DispatchData {
    transparent: bool,
    rename_all: RenameAllOption,
}

impl From<&EnumDispatchData> for DispatchData {
    fn from(data: &EnumDispatchData) -> Self {
        Self {
            transparent: false,
            rename_all: data.rename_all,
        }
    }
}

impl From<&StructDispatchData> for DispatchData {
    fn from(data: &StructDispatchData) -> Self {
        Self {
            transparent: data.transparent,
            rename_all: RenameAllOption::None,
        }
    }
}

fn parse_enum_dispatch_attributes(attributes: &[Attribute]) -> Result<EnumDispatchData, Error> {
    enum EnumDispatchAttribute {
        TagKey,
        RenameAll,
    }

    impl ParsedAttribute for EnumDispatchAttribute {
        fn from_path(path: &Path) -> Option<Self> {
            add_attribute_branch!(path, "tag_key", TagKey);
            add_attribute_branch!(path, "rename_all", RenameAll);
            None
        }
    }

    let mut tag_key = None;
    let mut rename_all = RenameAllOption::None;
    EnumDispatchAttribute::parse_attributes(attributes, |attribute, meta, ident| {
        match attribute {
            // tag_key = "x"
            EnumDispatchAttribute::TagKey => {
                if tag_key.is_some() {
                    return Err(duplicate_attribute_error(ident));
                }
                let value = meta.value()?;
                let lit = value.parse::<LitStr>()?;
                tag_key = Some(lit.value());
            }
            EnumDispatchAttribute::RenameAll => {
                if tag_key.is_some() {
                    return Err(duplicate_attribute_error(ident));
                }
                let value = meta.value()?;
                let lit = value.parse::<LitStr>()?;
                rename_all = RenameAllOption::try_from(&lit)?;
            }
        }
        Ok(())
    })?;
    let tag_key = tag_key.unwrap_or("type".to_string());
    Ok(EnumDispatchData {
        tag_key,
        rename_all,
    })
}

struct StructDispatchData {
    transparent: bool,
}

fn parse_struct_dispatch_attributes(attributes: &[Attribute]) -> Result<StructDispatchData, Error> {
    enum StructDispatchAttribute {
        Transparent,
    }

    impl ParsedAttribute for StructDispatchAttribute {
        fn from_path(path: &Path) -> Option<Self> {
            add_attribute_branch!(path, "transparent", Transparent);
            None
        }
    }

    let mut transparent = false;
    StructDispatchAttribute::parse_attributes(attributes, |attribute, _, ident| {
        match attribute {
            // tag_key = "x"
            StructDispatchAttribute::Transparent => {
                if transparent {
                    return Err(duplicate_attribute_error(ident));
                }
                transparent = true;
            }
        }
        Ok(())
    })?;
    Ok(StructDispatchData { transparent })
}

fn parse_enum_variant_attributes(
    ident: &Ident,
    attributes: &[Attribute],
    shared_dispatch_data: &DispatchData,
) -> Result<String, Error> {
    enum EnumVariantAttribute {
        Tag,
    }

    impl ParsedAttribute for EnumVariantAttribute {
        fn from_path(path: &Path) -> Option<Self> {
            add_attribute_branch!(path, "tag", Tag);
            None
        }
    }

    let mut ty = None;
    EnumVariantAttribute::parse_attributes(attributes, |attribute, meta, ident| {
        match attribute {
            // tag = "x"
            EnumVariantAttribute::Tag => {
                if ty.is_some() {
                    return Err(duplicate_attribute_error(ident));
                }
                let value = meta.value()?;
                let lit = value.parse::<LitStr>()?;
                ty = Some(lit.value());
            }
        }
        Ok(())
    })?;
    Ok(ty.unwrap_or_else(|| shared_dispatch_data.rename_all.apply(&ident.to_string())))
}

/// Expects an `Option` type, and if it is an `Option`, returns the type of the `Option` in a `Some`.
fn option_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(type_path) = ty
        && type_path.qself.is_none()
        && let Some(segment) = type_path.path.segments.last()
        && segment.ident == "Option"
    {
        let args = match &segment.arguments {
            syn::PathArguments::AngleBracketed(args) => &args.args,
            _ => return None,
        };

        match args.first()? {
            syn::GenericArgument::Type(inner_ty) => Some(inner_ty),
            _ => None,
        }
    } else {
        None
    }
}
