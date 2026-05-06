use pumpkin_codecs::{DataResult, Decode, DynamicOps, Encode};
use pumpkin_codecs::codec::{FieldEncode, MapEncode, optional_field::OptionalFieldEncode};
use pumpkin_codecs::struct_builder::StructBuilder;
use crate::text::{TextComponentBase, TextContent};

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

impl Encode for TextContent {
    fn encode<O: DynamicOps>(&self, ops: &'static O, prefix: O::Value) -> DataResult<O::Value> {
        let prefix = self.map_encode(ops, prefix);
        prefix
    }
}

impl MapEncode for TextContent {
    fn map_encode<O: DynamicOps, B: StructBuilder<Value=O::Value>>(&self, ops: &'static O, mut prefix: B) -> B {
        match self {
            Self::Text { text } => {
                prefix = text.encode_field("text", ops, prefix);
            }
            Self::Translate { translate, fallback, with, .. } => {
                prefix = translate.encode_field("translate", ops, prefix);
                prefix = fallback.encode_optional_field("fallback", ops, prefix);
                prefix = with.encode_field("with", ops, prefix);
            }
            Self::EntityNames { selector, separator } => {
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
}

impl Decode for TextContent {
    fn decode<O: DynamicOps>(input: O::Value, ops: &'static O) -> DataResult<(Self, O::Value)> {
        todo!()
    }
}

struct FuzzyTextContent(TextContent);

impl Encode for FuzzyTextContent {
    fn encode<O: DynamicOps>(&self, ops: &'static O, prefix: O::Value) -> DataResult<O::Value> {
        todo!()
    }
}
