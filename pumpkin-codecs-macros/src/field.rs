use crate::attribute::{ParsedAttribute, add_attribute_branch};
use crate::decode::DecodeModifier;
pub use crate::encode::EncodeModifier;
use crate::{duplicate_attribute_error, option_type};
use proc_macro_error2::__export::proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, quote};
use syn::{Attribute, Error, Field, Index, LitStr, Path, Token, Type};

/// Data from parsing a single field.
pub enum FieldData {
    /// Serialization occurs with the given field name.
    Present(Box<PresentFieldData>),
    /// Serialization of the field is ignored.
    Skipped { default: TokenStream },
}

pub struct PresentFieldData {
    pub name: String,
    pub lenient: bool,
    /// If `Some`, tells the specified default value of this field.
    pub default: Option<TokenStream>,
    /// If this is true, tells that the `default` attribute was specified,
    /// but no specific default value was set.
    pub implicit_default: bool,
    /// If this is true, inlines the fields encoded by this field into
    /// the parent's map while encoding.
    pub flatten: bool,
    /// Specifies an ordered list of modifiers to apply on a value before encoding it.
    pub encode_modifiers: Vec<EncodeModifier>,
    /// Specifies an ordered list of modifiers to apply, starting from the end, on a value after decoding it.
    pub decode_modifiers: Vec<DecodeModifier>,
    /// The final type, after all encode transformations or before all decode transformations, of the value to encode/decode.
    pub final_type: Option<Type>,
    /// Whether the field is the transparent field.
    pub transparent: bool,
}

/// Tells how a field is encoded/decoded.
pub enum FieldKind<'a> {
    Flatten,
    Option { ty: &'a Type },
    Defaulted { defaulted_tokens: TokenStream },
    Required,
    Transparent,
}

impl<'a> FieldKind<'a> {
    pub fn from_data(field: &ParsedField<'a>, data: &PresentFieldData) -> Self {
        if data.transparent {
            FieldKind::Transparent
        } else if data.flatten {
            FieldKind::Flatten
        } else if let Some(ty) = option_type(field.ty()) {
            FieldKind::Option { ty }
        } else if data.default.is_some() || data.implicit_default {
            FieldKind::Defaulted {
                defaulted_tokens: data
                    .default
                    .clone()
                    .unwrap_or_else(|| quote! {Default::default()}),
            }
        } else {
            FieldKind::Required
        }
    }
}

/// A [`Field`] reference wrapper to easily tell if the field
/// is named or not.
#[derive(Copy, Clone)]
pub enum ParsedField<'a> {
    Named(&'a Field),
    Unnamed(&'a Field, usize),
    Redirect {
        original: &'a Field,
        redirect_ident: &'a Ident,
        redirect_ty: &'a Type,
    },
}

/// A valid field attribute for the Encode and Decode trait derives.
pub enum ParsedFieldAttribute {
    Default,
    Lenient,
    Name,
    Skip,
    Flatten,
    Validate,
    As,
}

impl ParsedAttribute for ParsedFieldAttribute {
    fn from_path(path: &Path) -> Option<Self> {
        add_attribute_branch!(path, "default", Default);
        add_attribute_branch!(path, "lenient", Lenient);
        add_attribute_branch!(path, "name", Name);
        add_attribute_branch!(path, "skip", Skip);
        add_attribute_branch!(path, "flatten", Flatten);
        add_attribute_branch!(path, "validate", Validate);
        add_attribute_branch!(path, "as", As);
        None
    }
}

struct ParsedFieldAttributeData {
    field_name: Option<String>,
    default: Option<TokenStream>,
    implicit_default: bool,
    skipped: bool,
    lenient: bool,
    flatten: bool,
    encode_modifiers: Vec<EncodeModifier>,
    decode_modifiers: Vec<DecodeModifier>,
    final_type: Option<Type>,
}

impl<'a> ParsedField<'a> {
    /// Returns the name of this field as an `Ident`, as a reference, if any.
    pub const fn named_ident(self) -> Option<&'a Ident> {
        match self {
            Self::Named(f) => Some(f.ident.as_ref().unwrap()),
            Self::Unnamed(_, _) => None,
            Self::Redirect { redirect_ident, .. } => Some(redirect_ident),
        }
    }

    /// Returns the index of this field, if any.
    pub const fn index(&self) -> Option<usize> {
        match self {
            Self::Unnamed(_, i) => Some(*i),
            _ => None,
        }
    }

    /// Returns the `TokenStream` for accessing this field of a value.
    /// It can be an `Ident` or `Index`.
    pub fn access(self) -> TokenStream {
        match self {
            Self::Named(f) => f.ident.as_ref().unwrap().clone().into_token_stream(),
            Self::Unnamed(_, i) => Index::from(i).into_token_stream(),
            Self::Redirect { redirect_ident, .. } => redirect_ident.into_token_stream(),
        }
    }

    /// Returns the `Type`, as a reference, of this field.
    pub const fn ty(self) -> &'a Type {
        match self {
            Self::Named(f) | Self::Unnamed(f, _) => &f.ty,
            Self::Redirect { redirect_ty, .. } => redirect_ty,
        }
    }

    /// Returns a slice of the list of `Attribute`s of this field.
    pub fn attrs(self) -> &'a [Attribute] {
        match self {
            Self::Named(f) | Self::Unnamed(f, _) | Self::Redirect { original: f, .. } => &f.attrs,
        }
    }

    pub const fn into_redirect(self, redirect_ident: &'a Ident, redirect_ty: &'a Type) -> Self {
        Self::Redirect {
            original: match self {
                ParsedField::Named(f) | ParsedField::Unnamed(f, _) => f,
                ParsedField::Redirect { original, .. } => original,
            },
            redirect_ident,
            redirect_ty,
        }
    }

    /// Constructs a new `ParsedField` from a `Field`'s reference and the provided index,
    /// which may or may not be used.
    pub const fn from_field(value: &'a Field, index: usize) -> Self {
        if value.ident.is_some() {
            Self::Named(value)
        } else {
            Self::Unnamed(value, index)
        }
    }

    fn parse_and_set_bool(b: &mut bool, ident: &Ident) -> Result<(), Error> {
        if *b {
            return Err(duplicate_attribute_error(ident));
        }
        *b = true;
        Ok(())
    }

    /// Parses this field to get its [`FieldData`].
    pub fn generate_field_data(self, transparent: bool) -> Result<FieldData, Error> {
        let mut data = ParsedFieldAttributeData {
            field_name: None,
            default: None,
            implicit_default: false,
            skipped: false,
            lenient: false,
            flatten: false,
            encode_modifiers: Vec::new(),
            decode_modifiers: Vec::new(),
            final_type: None,
        };

        ParsedAttribute::parse_attributes(self.attrs(), |attribute, meta, ident| {
            match attribute {
                // default or default = ..
                ParsedFieldAttribute::Default => {
                    if data.default.is_some() {
                        return Err(duplicate_attribute_error(ident));
                    }
                    if meta.input.peek(Token![=]) {
                        let expr: syn::Expr = meta.value()?.parse()?;
                        data.default = Some(expr.into_token_stream());
                    } else {
                        data.default = None;
                        data.implicit_default = true;
                    }
                }
                // lenient
                ParsedFieldAttribute::Lenient => {
                    Self::parse_and_set_bool(&mut data.lenient, ident)?;
                }
                // name = "x"
                ParsedFieldAttribute::Name => {
                    if data.field_name.is_some() {
                        return Err(duplicate_attribute_error(ident));
                    }
                    let value = meta.value()?;
                    let lit = value.parse::<LitStr>()?;
                    data.field_name = Some(lit.value());
                }
                // skip
                ParsedFieldAttribute::Skip => Self::parse_and_set_bool(&mut data.skipped, ident)?,
                // flatten
                ParsedFieldAttribute::Flatten => {
                    Self::parse_and_set_bool(&mut data.flatten, ident)?;
                }
                // validate
                ParsedFieldAttribute::Validate => {
                    let path: Path = meta.value()?.parse()?;
                    data.encode_modifiers
                        .push(EncodeModifier::Validate(path.clone()));
                    data.decode_modifiers.push(DecodeModifier::Validate(path));
                }
                // as
                ParsedFieldAttribute::As => {
                    let ty: Type = meta.value()?.parse()?;
                    data.final_type = Some(ty);
                }
            }
            Ok(())
        })?;

        self.validate_parsed_attribute_data(transparent, data)
    }

    fn validate_parsed_attribute_data(
        self,
        transparent: bool,
        data: ParsedFieldAttributeData,
    ) -> Result<FieldData, Error> {
        if data.skipped {
            if data.field_name.is_some() || data.lenient || data.flatten {
                return Err(Error::new_spanned(
                    self.access(),
                    "Cannot specify this attribute for a skipped field",
                ));
            }
            // Default to using the Default trait if no specific default value is given.
            return Ok(FieldData::Skipped {
                default: data
                    .default
                    .unwrap_or_else(|| quote! { Default::default() }),
            });
        }

        if data.flatten && (data.default.is_some() || data.implicit_default) {
            return Err(Error::new_spanned(
                self.access(),
                "Cannot use `flatten` and `default` attributes together",
            ));
        }

        if data.flatten && (!data.encode_modifiers.is_empty() || !data.decode_modifiers.is_empty())
        {
            return Err(Error::new_spanned(
                self.access(),
                "Cannot use `flatten` with functional field attributes",
            ));
        }

        if data.final_type.is_none()
            && (!data
                .encode_modifiers
                .iter()
                .all(EncodeModifier::is_validate)
                || !data
                    .decode_modifiers
                    .iter()
                    .all(DecodeModifier::is_validate))
        {
            return Err(Error::new_spanned(
                self.access(),
                "A `final_type` needs to be specified if there is a functional attribute (other than `validate`)",
            ));
        }

        let name = if transparent {
            Some(String::new())
        } else {
            data.field_name
                .or_else(|| self.named_ident().map(ToString::to_string))
        };

        name.map_or_else(
            || {
                Err(Error::new_spanned(
                    self.access(),
                    "No field name could be inferred",
                ))
            },
            |name| {
                Ok(FieldData::Present(Box::new(PresentFieldData {
                    name,
                    lenient: data.lenient,
                    default: data.default,
                    implicit_default: data.implicit_default,
                    flatten: data.flatten,
                    encode_modifiers: data.encode_modifiers,
                    decode_modifiers: data.decode_modifiers,
                    final_type: data.final_type,
                    transparent,
                })))
            },
        )
    }
}
