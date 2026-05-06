use pumpkin_codecs::{DataResult, Decode, DynamicOps, Encode, };
use crate::text::{TextComponent, TextContent};

impl Encode for TextContent {
    fn encode<O: DynamicOps>(&self, ops: &'static O, prefix: O::Value) -> DataResult<O::Value> {
        match self {
            Self::Text { text } => {

            }
            Self::Translate { .. } => {}
            Self::EntityNames { .. } => {}
            Self::Keybind { .. } => {}
            Self::Custom { .. } => {}
        }
    }
}

impl MapEncode for TextContent {

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
