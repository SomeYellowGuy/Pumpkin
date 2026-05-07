use crate::field::{FieldData, FieldKind, ParsedField, PresentFieldData};
use crate::{
    DispatchData, parse_enum_dispatch_attributes, parse_enum_variant_attributes,
    parse_struct_dispatch_attributes,
};
use proc_macro::TokenStream;
use proc_macro_error2::__export::proc_macro2;
use proc_macro_error2::__export::proc_macro2::{Ident, Span};
use quote::{format_ident, quote};
use syn::{Attribute, Data, DataEnum, DataStruct, DeriveInput, Error, Fields, LitStr, Path};

pub fn derive_encode(
    codecs_crate: &proc_macro2::TokenStream,
    input: &DeriveInput,
) -> Result<TokenStream, Error> {
    let name = input.ident.clone();

    match &input.data {
        Data::Struct(data) => derive_struct_encode(&name, codecs_crate, data, &input.attrs),
        Data::Enum(data) => derive_enum_encode(&name, codecs_crate, data, &input.attrs),
        Data::Union(_) => Err(Error::new_spanned(
            input,
            "Only structs and enums are supported",
        )),
    }
}

/// Used to implement `Encode` for a type implementing `MapEncode`.
fn encode_delegate_impl(
    name: &Ident,
    codecs_crate: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote! {
        impl #codecs_crate::codec::Encode for #name {
            fn encode<O: #codecs_crate::DynamicOps>(&self, ops: &'static O, prefix: O::Value) -> #codecs_crate::DataResult<O::Value> {
                let mut builder = #codecs_crate::DynamicOps::map_builder(ops);
                builder = #codecs_crate::codec::MapEncode::map_encode(self, ops, builder);
                #codecs_crate::struct_builder::StructBuilder::build(builder, prefix)
            }
        }
    }
}

fn derive_struct_encode(
    name: &Ident,
    codecs_crate: &proc_macro2::TokenStream,
    data: &DataStruct,
    attrs: &[Attribute],
) -> Result<TokenStream, Error> {
    // Add a special case for unit structs.
    if matches!(&data.fields, Fields::Unit) {
        return Ok(quote! {
            impl #codecs_crate::codec::Encode for #name {
                fn encode<O: #codecs_crate::DynamicOps>(&self, ops: &'static O, prefix: O::Value) -> #codecs_crate::DataResult<O::Value> {
                    #codecs_crate::DynamicOps::merge_map_like_into_map(ops, prefix, #codecs_crate::EmptyMapLike::new())
                }
            }
        }.into());
    }
    let dispatch_data = parse_struct_dispatch_attributes(attrs)?;
    let variant_encode =
        derive_single_variant_encode(codecs_crate, &data.fields, &(&dispatch_data).into());
    let encode_impl = encode_delegate_impl(name, codecs_crate);
    if dispatch_data.transparent {
        Ok(
            quote! {
                impl #codecs_crate::codec::Encode for #name {
                    fn encode<O: #codecs_crate::DynamicOps>(&self, ops: &'static O, prefix: O::Value) -> #codecs_crate::DataResult<O::Value> {
                        #variant_encode
                    }
                }
            }.into()
        )
    } else {
        Ok(
            quote! {
            impl #codecs_crate::codec::MapEncode for #name {
                fn map_encode<O: #codecs_crate::DynamicOps, B: #codecs_crate::struct_builder::StructBuilder<Value=O::Value>>(&self, ops: &'static O, mut builder: B) -> B {
                    #variant_encode
                    builder
                }
            }

            #encode_impl
        }.into()
        )
    }
}

fn derive_enum_encode(
    name: &Ident,
    codecs_crate: &proc_macro2::TokenStream,
    data: &DataEnum,
    attrs: &[Attribute],
) -> Result<TokenStream, Error> {
    let dispatch_data = parse_enum_dispatch_attributes(attrs)?;
    let shared = &(&dispatch_data).into();

    // Add a special case for all variants being unit variants.
    if data
        .variants
        .iter()
        .all(|v| matches!(v.fields, Fields::Unit))
    {
        // We encode all variants as strings.
        let mut match_arms = Vec::new();
        for variant in &data.variants {
            let ident = &variant.ident;
            let ty = parse_enum_variant_attributes(&variant.ident, &variant.attrs, shared)?;
            let ty_lit = LitStr::new(&ty, Span::call_site());
            match_arms.push(quote! {
                Self::#ident => #ty_lit
            });
        }
        return Ok(
            quote! {
                impl #codecs_crate::codec::Encode for #name {
                    fn encode<O: #codecs_crate::DynamicOps>(&self, ops: &'static O, prefix: O::Value) -> #codecs_crate::DataResult<O::Value> {
                        let string = match self { #( #match_arms ),* }.to_string();
                        #codecs_crate::codec::Encode::encode(&string, ops, prefix)
                    }
                }
            }.into()
        );
    }
    let tag_key_lit = LitStr::new(&dispatch_data.tag_key, Span::call_site());

    let mut match_arms = Vec::new();
    for variant in &data.variants {
        let ty = parse_enum_variant_attributes(&variant.ident, &variant.attrs, shared)?;
        let ty_lit = LitStr::new(&ty, Span::call_site());

        let fields: Vec<_> = variant
            .fields
            .iter()
            .enumerate()
            .map(|(i, f)| {
                f.ident
                    .as_ref()
                    .map_or_else(|| format_ident!("a{i}"), Clone::clone)
            })
            .collect();
        let ident = &variant.ident;
        let mat = match variant.fields {
            Fields::Named(_) => Some(quote! { { #( #fields ),* } }),
            Fields::Unnamed(_) => Some(quote! { ( #( #fields ),* ) }),
            Fields::Unit => None,
        };
        let variant_encode = derive_single_variant_builder_encode(
            codecs_crate,
            &variant.fields,
            |f| {
                if matches!(&variant.fields, Fields::Unnamed(_)) {
                    let ident = format_ident!("a{}", f.index().unwrap());
                    quote! { #ident }
                } else {
                    let ident = f.named_ident().unwrap();
                    quote! { #ident }
                }
            },
            &(&dispatch_data).into(),
        );
        match_arms.push(quote! {
            Self::#ident #mat => {
                builder = #codecs_crate::struct_builder::StructBuilder::add_string_key_value(builder, #tag_key_lit, ops.create_string(#ty_lit));
                #variant_encode
            }
        });
    }

    let encode_impl = encode_delegate_impl(name, codecs_crate);

    Ok(
        quote! {
            impl #codecs_crate::codec::MapEncode for #name {
                fn map_encode<O: #codecs_crate::DynamicOps, B: #codecs_crate::struct_builder::StructBuilder<Value=O::Value>>(&self, ops: &'static O, mut builder: B) -> B {
                    match self {
                        #( #match_arms ),*
                    }
                    builder
                }
            }

            #encode_impl
        }.into()
    )
}

/// Creates a single variant's encoding in tokens.
fn derive_single_variant_encode(
    codecs_crate: &proc_macro2::TokenStream,
    fields: &Fields,
    shared_dispatch_data: &DispatchData,
) -> proc_macro2::TokenStream {
    derive_single_variant_builder_encode(
        codecs_crate,
        fields,
        |f| {
            let access = f.access();
            quote! { &self. #access }
        },
        shared_dispatch_data,
    )
}

/// Creates a single variant's encoding in tokens.
fn derive_single_variant_builder_encode(
    codecs_crate: &proc_macro2::TokenStream,
    fields: &Fields,
    access_fn: impl Fn(&ParsedField) -> proc_macro2::TokenStream,
    shared_dispatch_data: &DispatchData,
) -> proc_macro2::TokenStream {
    let mut builder_encodes = Vec::new();
    if shared_dispatch_data.transparent && fields.len() != 1 {
        return Error::new(
            Span::call_site(),
            "A struct with the `transparent` attribute can only have 1 field",
        )
        .to_compile_error();
    }
    for (index, field) in fields.iter().enumerate() {
        let field = ParsedField::from_field(field, index);
        match encode_field_tokens(codecs_crate, field, &access_fn, shared_dispatch_data) {
            Ok(EncodeFieldData { builder_encode }) => {
                builder_encodes.push(builder_encode);
            }
            Err(e) => return e.to_compile_error(),
        }
    }
    quote! { #(#builder_encodes)* }
}

struct EncodeFieldData {
    builder_encode: Option<proc_macro2::TokenStream>,
}

/// A modifier applied to a value before encoding.
pub enum EncodeModifier {
    Validate(Path),
}

impl EncodeModifier {
    pub const fn is_validate(&self) -> bool {
        matches!(self, Self::Validate(_))
    }
}

impl EncodeModifier {
    fn generate(&self, codecs_crate: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        match self {
            Self::Validate(p) => quote! {
                let r = #codecs_crate::DataResult::flat_map(r, |r| #p(r).map_or_else(#codecs_crate::DataResult::new_error, |()| #codecs_crate::DataResult::new_success(r)));
            },
        }
    }
}

fn encode_field_tokens(
    codecs_crate: &proc_macro2::TokenStream,
    field: ParsedField,
    access_fn: impl Fn(&ParsedField) -> proc_macro2::TokenStream,
    shared_dispatch_data: &DispatchData,
) -> Result<EncodeFieldData, Error> {
    let data = field.generate_field_data(shared_dispatch_data.transparent)?;
    match data {
        FieldData::Present(data) => Ok(encode_from_field_data(
            codecs_crate,
            field,
            *data,
            access_fn,
        )),
        FieldData::Skipped { .. } => Ok(EncodeFieldData {
            builder_encode: None,
        }),
    }
}

fn encode_from_field_data(
    codecs_crate: &proc_macro2::TokenStream,
    field: ParsedField,
    mut data: PresentFieldData,
    access_fn: impl Fn(&ParsedField) -> proc_macro2::TokenStream,
) -> EncodeFieldData {
    let access = access_fn(&field);
    let encoded_name_lit = LitStr::new(&data.name, Span::call_site());
    let kind = FieldKind::from_data(&field, &data);
    let builder_encode = if data.encode_modifiers.is_empty() {
        // If there are no encode modifiers, use the simple trait functions.
        match kind {
            FieldKind::Flatten => quote! {
                builder = #codecs_crate::codec::MapEncode::map_encode(#access, ops, builder);
            },
            FieldKind::Option { .. } => quote! {
                builder = #codecs_crate::codec::optional_field::OptionalFieldEncode::encode_optional_field(#access, #encoded_name_lit, ops, builder);
            },
            FieldKind::Defaulted { defaulted_tokens } => {
                quote! {
                    builder = #codecs_crate::codec::FieldEncode::encode_defaulted_field(#access, #encoded_name_lit, ops, builder, #defaulted_tokens);
                }
            }
            FieldKind::Required => quote! {
                builder = #codecs_crate::codec::FieldEncode::encode_field(#access, #encoded_name_lit, ops, builder);
            },
            FieldKind::Transparent => quote! {
                #codecs_crate::codec::Encode::encode_start(#access, ops)
            },
        }
    } else {
        // Otherwise, we apply transformations to the value.
        //
        // We start with a `DataResult` success and keep mapping/flat mapping it with functions until
        // we get the desired value to encode.
        let mut transformations = Vec::new();
        for modifier in &data.encode_modifiers {
            let transformation = modifier.generate(codecs_crate);
            transformations.push(transformation);
        }
        data.encode_modifiers.clear();
        let g = &data.final_type.as_ref().unwrap_or_else(|| field.ty());
        let builder_encode_end = match FieldKind::from_data(
            &field.into_redirect(&format_ident!("r"), g),
            &data,
        ) {
            // `flatten` does not work with any modifiers.
            FieldKind::Flatten => unimplemented!(),
            FieldKind::Option { .. } => quote! {
                builder = #codecs_crate::struct_builder::StructBuilder::with_errors_from(builder, &r);
                builder = match r {
                    #codecs_crate::DataResult::Success { result, .. } => if let Some(value) = result {
                        #codecs_crate::codec::FieldEncode::encode_field(value, #encoded_name_lit, ops, builder)
                    } else {
                        builder
                    },
                    #codecs_crate::DataResult::Error { .. } => builder
                };
            },
            FieldKind::Defaulted { defaulted_tokens } => quote! {
                builder = #codecs_crate::struct_builder::StructBuilder::with_errors_from(builder, &r);
                builder = match r {
                    #codecs_crate::DataResult::Success { result, .. } => if #defaulted_tokens == *result {
                        builder
                    } else {
                        #codecs_crate::codec::FieldEncode::encode_field(result, #encoded_name_lit, ops, builder)
                    },
                    #codecs_crate::DataResult::Error { .. } => builder
                };
            },
            FieldKind::Required => quote! {
                builder = #codecs_crate::struct_builder::StructBuilder::add_string_key_value_result(
                    #encoded_name_lit, #codecs_crate::DataResult::flat_map(r, |r| #codecs_crate::codec::Encode::encode_start(r, ops))
                );
            },
            FieldKind::Transparent => quote! {
                #codecs_crate::DataResult::flat_map(r, |r| #codecs_crate::codec::Encode::encode_start(r, ops))
            },
        };
        quote! {
            let r = #codecs_crate::DataResult::new_success(#access);
            #(#transformations)*
            #builder_encode_end
        }
    };

    EncodeFieldData {
        builder_encode: Some(builder_encode),
    }
}
