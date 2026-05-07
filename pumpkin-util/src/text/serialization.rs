use crate::text::{TextComponentBase, TextContent};
use crate::translation::Locale;
use pumpkin_codecs::codec::optional_field::OptionalFieldDecode;
use pumpkin_codecs::codec::{FieldDecode, FieldEncode, optional_field::OptionalFieldEncode, MapEncode, MapDecode};
use pumpkin_codecs::struct_builder::StructBuilder;
use pumpkin_codecs::{DataResult, Decode, DynamicOps, Encode, MapLike};
use std::borrow::Cow;

impl Encode for TextComponentBase {
    fn encode<O: DynamicOps>(&self, ops: &'static O, prefix: O::Value) -> DataResult<O::Value> {
        todo!()
    }
}

impl Decode for TextComponentBase {
    fn decode<O: DynamicOps>(input: O::Value, ops: &'static O) -> DataResult<(Self, O::Value)> {
        todo!()
    }
}

impl TextContent {
    fn map_encode_specific<O: DynamicOps, B: StructBuilder<Value = O::Value>>(
        &self,
        ops: &'static O,
        mut prefix: B,
    ) -> B {
        match self {
            Self::Text { text } => {
                prefix = text.encode_field("text", ops, prefix);
            }
            Self::Translate {
                translate,
                fallback,
                with,
                ..
            } => {
                prefix = translate.encode_field("translate", ops, prefix);
                prefix = fallback.encode_optional_field("fallback", ops, prefix);
                prefix = with.encode_field("with", ops, prefix);
            }
            Self::EntityNames {
                selector,
                separator,
            } => {
                prefix = selector.encode_field("selector", ops, prefix);
                prefix = separator.encode_optional_field("separator", ops, prefix);
            }
            Self::Keybind { keybind } => {
                prefix = keybind.encode_field("keybind", ops, prefix);
            }
            Self::Custom { key, locale, with } => {
                prefix = key.encode_field("key", ops, prefix);
                prefix = locale.encode_field("locale", ops, prefix);
                prefix = with.encode_field("with", ops, prefix);
            }
        }
        prefix
    }

    fn map_decode_specific<O: DynamicOps>(
        ty: TextContentType,
        input: &impl MapLike<Value = O::Value>,
        ops: &'static O,
    ) -> DataResult<Self> {
        match ty {
            TextContentType::Text => {
                let text = Cow::<'static, str>::decode_field("text", input, ops);
                text.map(|text| Self::Text { text })
            }
            TextContentType::Translate => {
                let translate = Cow::<'static, str>::decode_field("translate", input, ops);
                let fallback = Option::<Cow<'static, str>>::decode_optional_field(
                    "fallback", input, ops, true,
                );
                // TODO: accept many other things as well here
                let with = Vec::<TextComponentBase>::decode_field("with", input, ops);
                translate.apply_3(
                    |translate, fallback, with| Self::Translate {
                        translate,
                        fallback,
                        with,
                        bedrock_translate: None,
                    },
                    fallback,
                    with,
                )
            }
            TextContentType::EntityNames => {
                let selector = Cow::<'static, str>::decode_field("selector", input, ops);
                let separator = Option::<Cow<'static, str>>::decode_optional_field(
                    "separator",
                    input,
                    ops,
                    false,
                );
                selector.apply_2(
                    |selector, separator| Self::EntityNames {
                        selector,
                        separator,
                    },
                    separator,
                )
            }
            TextContentType::Keybind => {
                let keybind = Cow::<'static, str>::decode_field("keybind", input, ops);
                keybind.map(|keybind| Self::Keybind { keybind })
            }
            TextContentType::Custom => {
                let key = Cow::<'static, str>::decode_field("key", input, ops);
                let locale = Locale::decode_field("locale", input, ops);
                let with = Vec::<TextComponentBase>::decode_field("with", input, ops);
                key.apply_3(
                    |key, locale, with| Self::Custom { key, locale, with },
                    locale,
                    with,
                )
            }
        }
    }
}

enum TextContentType {
    Text,
    Translate,
    EntityNames,
    Keybind,
    Custom,
}

impl TextContentType {
    const ALL: [Self; 5] = [
        Self::Text,
        Self::Translate,
        Self::EntityNames,
        Self::Keybind,
        Self::Custom,
    ];
}

// Fuzzy
impl TextContent {
    fn fuzzy_map_encode<O: DynamicOps, B: StructBuilder<Value=O::Value>>(&self, ops: &'static O, prefix: B) -> B {
        self.map_encode_specific(ops, prefix)
    }

    fn fuzzy_map_decode<O: DynamicOps>(input: &impl MapLike<Value=O::Value>, ops: &'static O) -> DataResult<Self> {
        for ty in TextContentType::ALL {
            let result = TextContent::map_decode_specific(ty, input, ops);
            if result.is_success() {
                return result
            }
        }
        DataResult::new_error("No matching codec found")
    }
}

// Legacy
impl TextContent {
    fn legacy_map_decode<O: DynamicOps>(input: &impl MapLike<Value=O::Value>, ops: &'static O) -> DataResult<Self> {
        let ty = String::decode_field("type", input, ops);
        ty.flat_map(|s| {
            match s.as_str() {
                "text" => TextContent::map_decode_specific(TextContentType::Text, input, ops),
                "translate" => TextContent::map_decode_specific(TextContentType::Translate, input, ops),
                "selector" => TextContent::map_decode_specific(TextContentType::EntityNames, input, ops),
                "keybind" => TextContent::map_decode_specific(TextContentType::Keybind, input, ops),
                "custom" => TextContent::map_decode_specific(TextContentType::Custom, input, ops),
                _ => DataResult::new_error(format!("Unknown element id: {s}"))
            }
        })
    }
}

// Combined
impl MapEncode for TextContent {
    fn map_encode<O: DynamicOps, B: StructBuilder<Value=O::Value>>(&self, ops: &'static O, prefix: B) -> B {
        self.fuzzy_map_encode(ops, prefix)
    }
}

impl MapDecode for TextContent {
    fn map_decode<O: DynamicOps>(input: &impl MapLike<Value=O::Value>, ops: &'static O) -> DataResult<Self> {
        if input.get_str("type").is_some() {
            Self::legacy_map_decode(input, ops)
        } else {
            Self::fuzzy_map_decode(input, ops)
        }
    }
}
