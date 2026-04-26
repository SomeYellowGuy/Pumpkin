use crate::field::{FieldData, FieldKind, ParsedField, PresentFieldData};
use crate::{
    DispatchData, parse_enum_dispatch_attributes, parse_enum_variant_attributes,
    parse_struct_dispatch_attributes,
};
use proc_macro::TokenStream;
use proc_macro_error2::__export::proc_macro2;
use proc_macro_error2::__export::proc_macro2::Span;
use quote::{ToTokens, format_ident, quote};
use syn::{
    Attribute, Data, DataEnum, DataStruct, DeriveInput, Error, Fields, Ident, LitBool, LitStr, Path,
};

pub fn derive_decode(
    codecs_crate: &proc_macro2::TokenStream,
    input: &DeriveInput,
) -> Result<TokenStream, Error> {
    let name = input.ident.clone();

    match &input.data {
        Data::Struct(data) => derive_struct_decode(&name, codecs_crate, data, &input.attrs),
        Data::Enum(data) => derive_enum_decode(&name, codecs_crate, data, &input.attrs),
        Data::Union(_) => Err(Error::new_spanned(
            input,
            "Only structs and enums are supported",
        )),
    }
}

/// Used to implement `Decode` for a type implementing `MapDecode`.
fn decode_delegate_impl(
    name: &Ident,
    codecs_crate: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote! {
        impl #codecs_crate::codec::Decode for #name {
            fn decode<O: #codecs_crate::DynamicOps>(input: O::Value, ops: &'static O) -> #codecs_crate::DataResult<(Self, O::Value)> {
                let map = #codecs_crate::DynamicOps::get_map(ops, &input);
                let single_result = #codecs_crate::DataResult::with_lifecycle(map, #codecs_crate::Lifecycle::Stable)
                    .flat_map(|map| {
                        #codecs_crate::codec::MapDecode::map_decode(&map, ops)
                });
                #codecs_crate::DataResult::map(single_result, |s| (s, input))
            }
        }
    }
}

fn derive_struct_decode(
    name: &Ident,
    codecs_crate: &proc_macro2::TokenStream,
    data: &DataStruct,
    attrs: &[Attribute],
) -> Result<TokenStream, Error> {
    // Add a special case for unit structs.
    if matches!(&data.fields, Fields::Unit) {
        return Ok(
            quote! {
                impl #codecs_crate::codec::Decode for #name {
                    fn decode<O: #codecs_crate::DynamicOps>(input: O::Value, ops: &'static O) -> #codecs_crate::DataResult<(Self, O::Value)> {
                        let map = #codecs_crate::DynamicOps::get_map(ops, &input);
                        let result = #codecs_crate::DataResult::map(map, |_| ());
                        #codecs_crate::DataResult::map(result, |()| (Self, input))
                    }
                }
            }
            .into()
        );
    }
    let dispatch_data = parse_struct_dispatch_attributes(attrs)?;
    let variant_decode = derive_single_variant_decode(
        codecs_crate,
        name,
        &data.fields,
        &quote! { Self },
        &(&dispatch_data).into(),
    );

    let decode_impl = decode_delegate_impl(name, codecs_crate);
    if dispatch_data.transparent {
        Ok(
            quote! {
                impl #codecs_crate::codec::Decode for #name {
                    fn decode<O: #codecs_crate::DynamicOps>(input: O::Value, ops: &'static O) -> #codecs_crate::DataResult<(Self, O::Value)> {
                        #variant_decode
                    }
                }
            }.into()
        )
    } else {
        Ok(quote! {
            impl #codecs_crate::codec::MapDecode for #name {
                fn map_decode<O: #codecs_crate::DynamicOps>(
                        map: &impl #codecs_crate::MapLike<Value = O::Value>,
                        ops: &'static O,
                    ) -> #codecs_crate::DataResult<Self> {
                    #variant_decode
                }
            }

            #decode_impl
        }
        .into())
    }
}

fn derive_enum_decode(
    name: &Ident,
    codecs_crate: &proc_macro2::TokenStream,
    data: &DataEnum,
    attrs: &[Attribute],
) -> Result<TokenStream, Error> {
    let dispatch_data = parse_enum_dispatch_attributes(attrs)?;

    // Add a special case for all variants being unit variants.
    if data
        .variants
        .iter()
        .all(|v| matches!(v.fields, Fields::Unit))
    {
        let mut match_arms = Vec::new();
        for variant in &data.variants {
            let ident = &variant.ident;
            let ty = parse_enum_variant_attributes(&variant.ident, &variant.attrs, &(&dispatch_data).into())?;
            let ty_lit = LitStr::new(&ty, Span::call_site());
            match_arms.push(quote! {
                #ty_lit => #codecs_crate::DataResult::new_success((Self::#ident, p))
            });
        }
        return Ok(
            quote! {
                impl #codecs_crate::codec::Decode for #name {
                    fn decode<O: #codecs_crate::DynamicOps>(input: O::Value, ops: &'static O) -> #codecs_crate::DataResult<(Self, O::Value)> {
                        let string: #codecs_crate::DataResult<(String, O::Value)> = #codecs_crate::codec::Decode::decode(input, ops);
                        string.flat_map(|(s, p)| {
                            match s.as_str() {
                                #( #match_arms ),* ,
                                _ => #codecs_crate::DataResult::new_error(format!("Invalid type '{s}'"))
                            }
                        })
                    }
                }
            }.into()
        );
    }

    let tag_key_lit = LitStr::new(&dispatch_data.tag_key, Span::call_site());
    let mut match_arms = Vec::new();
    for variant in &data.variants {
        // Try to get the variant's differentiator value first.
        let ty = parse_enum_variant_attributes(&variant.ident, &variant.attrs, &(&dispatch_data).into())?;
        let ty_lit = LitStr::new(&ty, Span::call_site());
        let ident = &variant.ident;
        let qualified_variant_ident = quote! { Self::#ident };
        let variant_decode = if variant.fields.is_empty() {
            quote! { #codecs_crate::DataResult::new_success(#qualified_variant_ident) }
        } else {
            derive_single_variant_decode(
                codecs_crate,
                name,
                &variant.fields,
                &qualified_variant_ident,
                &(&dispatch_data).into(),
            )
        };
        match_arms.push(quote! {
            #ty_lit => {
                #variant_decode
            }
        });
    }
    let decode_impl = decode_delegate_impl(name, codecs_crate);
    Ok(
        quote! {
            impl #codecs_crate::codec::MapDecode for #name {
                fn map_decode<O: #codecs_crate::DynamicOps>(
                    map: &impl #codecs_crate::MapLike<Value = O::Value>,
                    ops: &'static O,
                ) -> #codecs_crate::DataResult<Self> {
                    let ty: #codecs_crate::DataResult<String> = #codecs_crate::codec::FieldDecode::decode_field::<O>(#tag_key_lit, map, ops);
                    ty.flat_map(|ty| {
                        match ty.as_str() {
                            #( #match_arms ),*
                            _ => #codecs_crate::DataResult::new_error(format!("Invalid differentiator key {ty}"))
                        }
                    })
                }
            }

            #decode_impl
        }.into()
    )
}

/// Creates a single variant's decoding in tokens.
fn derive_single_variant_decode(
    codecs_crate: &proc_macro2::TokenStream,
    variant_ident: &Ident,
    fields: &Fields,
    variant_tokens: &proc_macro2::TokenStream,
    shared_dispatch_data: &DispatchData,
) -> proc_macro2::TokenStream {
    if shared_dispatch_data.transparent && fields.len() != 1 {
        return Error::new(
            Span::call_site(),
            "A struct with the `transparent` attribute can only have 1 field",
        )
        .to_compile_error();
    }

    let mut builder_decodes = Vec::new();
    // The counted encoded values.
    let mut counter = 0;
    let mut field_inputs = Vec::new();
    let mut field_outputs = Vec::new();
    for (index, field) in fields.iter().enumerate() {
        let field = ParsedField::from_field(field, index);
        match decode_field_tokens(codecs_crate, field, &mut counter, shared_dispatch_data) {
            Ok(DecodeFieldData {
                builder_decode,
                field_input,
                field_output,
            }) => {
                builder_decodes.push(builder_decode);
                if let Some(input) = field_input {
                    field_inputs.push(input);
                }
                field_outputs.push(field_output);
            }
            Err(e) => return e.to_compile_error(),
        }
    }
    if counter < 1 {
        // TODO
        return Error::new_spanned(variant_ident, "At least 1 field must be decoded")
            .to_compile_error();
    } else if counter > 16 {
        return Error::new_spanned(variant_ident, "No more than 16 fields may be decoded")
            .to_compile_error();
    }
    let constructor_tokens = if shared_dispatch_data.transparent {
        match fields {
            Fields::Named(_) => quote! {
                |a| (#variant_tokens {a}, #codecs_crate::DynamicOps::empty(ops))
            },
            Fields::Unnamed(_) => quote! {
                |a| (#variant_tokens (a), #codecs_crate::DynamicOps::empty(ops))
            },
            Fields::Unit => unreachable!(),
        }
    } else {
        match fields {
            Fields::Named(_) => quote! {
                |#( #field_inputs ),*| #variant_tokens {#( #field_outputs ),*}
            },
            Fields::Unnamed(_) => quote! {
                |#( #field_inputs ),*| #variant_tokens (#( #field_outputs ),*)
            },
            Fields::Unit => quote! {
                || #variant_tokens
            },
        }
    };
    let apply_fn = if counter == 1 {
        format_ident!("map")
    } else {
        format_ident!("apply_{}", counter)
    };
    let other_apply_params = (1..counter).map(|i| format_ident!("a{i}"));
    quote! {
        #(#builder_decodes)*
        a0.#apply_fn( #constructor_tokens, #( #other_apply_params ),* )
    }
}

struct DecodeFieldData {
    /// The statement to decode a value from a map.
    builder_decode: Option<proc_macro2::TokenStream>,
    /// A constructor input in the `apply_n` or `map` function.
    field_input: Option<proc_macro2::TokenStream>,
    /// A value used to initialize the struct in the `apply_n` or `map` function.
    field_output: proc_macro2::TokenStream,
}

/// A modifier applied to a value after decoding.
pub enum DecodeModifier {
    Validate(Path),
}

impl DecodeModifier {
    pub const fn is_validate(&self) -> bool {
        matches!(self, Self::Validate(_))
    }
}

impl DecodeModifier {
    fn generate(
        &self,
        codecs_crate: &proc_macro2::TokenStream,
        ident: &Ident,
    ) -> proc_macro2::TokenStream {
        match self {
            Self::Validate(p) => quote! {
                let #ident = #codecs_crate::DataResult::flat_map(#ident, |r| #p(&r).map_or_else(#codecs_crate::DataResult::new_error, |()| #codecs_crate::DataResult::new_success(r)));
            },
        }
    }
}

fn decode_from_field_data(
    codecs_crate: &proc_macro2::TokenStream,
    field: ParsedField,
    mut data: PresentFieldData,
    counter: &mut usize,
    ident: Option<&Ident>,
) -> Result<DecodeFieldData, Error> {
    let encoded_name_lit = LitStr::new(&data.name, Span::call_site());
    let kind = FieldKind::from_data(&field, &data);
    let decoded_ident = format_ident!("a{counter}");
    let constructor_ident = ident.unwrap_or(&decoded_ident);
    let builder_decode = if data.decode_modifiers.is_empty() {
        *counter += 1;
        match kind {
            FieldKind::Flatten => quote! {
                let #decoded_ident = #codecs_crate::codec::MapDecode::map_decode(map, ops);
            },
            FieldKind::Option { ty } => {
                // For an Option, it can be lenient.
                let lenient_token = LitBool::new(data.lenient, Span::call_site());
                quote! {
                    let #decoded_ident: #codecs_crate::DataResult<Option<#ty>> = #codecs_crate::codec::optional_field::OptionalFieldDecode::decode_optional_field::<O>(#encoded_name_lit, map, ops, #lenient_token);
                }
            }
            FieldKind::Defaulted { defaulted_tokens } => {
                let lenient_token = LitBool::new(data.lenient, Span::call_site());
                let ty = field.ty();
                quote! {
                    let #decoded_ident: #codecs_crate::DataResult<#ty> = #codecs_crate::codec::FieldDecode::decode_defaulted_field::<O>(#encoded_name_lit, map, ops, #defaulted_tokens, #lenient_token);
                }
            }
            FieldKind::Required => {
                if data.lenient {
                    return Err(Error::new_spanned(field.ty(), "Invalid use of `lenient`"));
                }
                quote! {
                    let #decoded_ident = #codecs_crate::codec::FieldDecode::decode_field::<O>(#encoded_name_lit, map, ops);
                }
            }
            FieldKind::Transparent => quote! {
                let a0 = #codecs_crate::codec::Decode::parse::<O>(input, ops);
            },
        }
    } else {
        // Otherwise, we apply transformations to the value.
        //
        // We start with a `DataResult` success and keep mapping/flat mapping it with functions until
        // we get the desired value to encode.
        let mut transformations = Vec::new();
        for modifier in data.decode_modifiers.iter().rev() {
            let transformation = modifier.generate(codecs_crate, &decoded_ident);
            transformations.push(transformation);
        }
        data.decode_modifiers.clear();
        let l = decode_from_field_data(
            codecs_crate,
            field.into_redirect(&format_ident!("r"), field.ty()),
            data,
            counter,
            Some(&decoded_ident),
        )?;
        let inner_builder_decode = l.builder_decode;
        quote! {
            #inner_builder_decode
            #(#transformations)*
        }
    };
    Ok(DecodeFieldData {
        builder_decode: Some(builder_decode),
        field_input: Some(constructor_ident.clone().into_token_stream()),
        field_output: constructor_ident.into_token_stream(),
    })
}

fn decode_field_tokens(
    codecs_crate: &proc_macro2::TokenStream,
    field: ParsedField,
    counter: &mut usize,
    shared_dispatch_data: &DispatchData,
) -> Result<DecodeFieldData, Error> {
    let ident = field.named_ident();
    match field.generate_field_data(shared_dispatch_data.transparent)? {
        FieldData::Present(data) => {
            decode_from_field_data(codecs_crate, field, *data, counter, ident)
        }
        FieldData::Skipped { default } => {
            let default_tokens =
                ident.map_or_else(|| quote! { #default }, |ident| quote! { #ident: #default });
            Ok(DecodeFieldData {
                builder_decode: None,
                field_input: None,
                field_output: default_tokens,
            })
        }
    }
}
