use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use pumpkin_codecs::{DataResult, DynamicOps, Encode};
use pumpkin_codecs::codec::{FieldEncode, MapEncode};
use pumpkin_codecs::codec::optional_field::OptionalFieldEncode;
use pumpkin_codecs::struct_builder::StructBuilder;
use pumpkin_codecs_macros::{Decode, Encode};
use crate::uuid_util::LenientUuid;
use super::{TextComponent, TextComponentBase};

/// Represents the hover event action in a chat component.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum HoverEvent {
    /// Displays a tooltip with the given text.
    ShowText { value: TextComponentBase },
    /// Shows an item.
    ShowItem {
        /// Resource identifier of the item.
        id: Cow<'static, str>,
        /// Number of the items in the stack.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        count: Option<i32>,
        // #[serde(default, skip_serializing_if = "Option::is_none")]
        // components: Option<Cow<'static, str>>,
    },
    /// Shows an entity.
    ShowEntity {
        /// The entity's ID Entity Type.
        id: Cow<'static, str>,
        /// The entity's UUID
        /// The UUID cannot use `uuid::Uuid` because its serialization parses it into bytes, so its double bytes serialized.
        uuid: Cow<'static, str>,
        /// Optional custom name for the entity.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<TextComponentBase>,
    },
}

#[derive(Clone, Debug, Encode, Decode)]
#[codec(tag_key = "action")]
pub enum CodecHoverEvent {
    ShowText { value: TextComponentBase },
    ShowItem {
        id: Cow<'static, str>,
        #[codec(validate = CodecHoverEvent::validate_stack_count)]
        count: Option<i32>,
        // components: Option<Cow<'static, str>>,
    },
    ShowEntity {
        id: Cow<'static, str>,
        uuid: LenientUuid,
        name: Option<TextComponentBase>,
    },
}

impl CodecHoverEvent {
    fn validate_stack_count(count: &Option<i32>) -> Result<(), String> {
        if count.is_none_or(|c| (1..99).contains(&c)) {
            Ok(())
        } else {
            Err(format!("Value must be within range [1;99]: {}", count.unwrap()))
        }
    }
}

impl MapEncode for HoverEvent {
    fn map_encode<O: DynamicOps, B: StructBuilder<Value=O::Value>>(&self, ops: &'static O, mut prefix: B) -> B {
        let ty = match self {
            HoverEvent::ShowText { value } => {
                prefix = value.encode_field("value", ops, prefix);
                "show_text"
            }
            HoverEvent::ShowItem { id, count } => {
                prefix = id.encode_field("id", ops, prefix);
                prefix = Self::validate_stack_count(ops, prefix, *count);
                prefix = count.encode_optional_field("count", ops, prefix);
                "show_item"
            }
            HoverEvent::ShowEntity { id, uuid, name } => {
                prefix = id.encode_field("id", ops, prefix);
                let lenient_uuid = L
                prefix = uuid.encode_field("uuid", ops, prefix);
                prefix = name.encode_optional_field("name", ops, prefix);
                "show_entity"
            }
        };
        ty.to_string().encode_field("type", ops, prefix)
    }
}

impl HoverEvent {
    /// Creates a new hover event that displays text.
    ///
    /// # Arguments
    /// - `text` – The text component to display in the tooltip.
    ///
    /// # Returns
    /// A `HoverEvent::ShowText` variant containing the provided text.
    #[must_use]
    pub fn show_text(text: TextComponent) -> Self {
        Self::ShowText {
            value: text.0,
        }
    }

    /// Creates a new hover event that displays entity information.
    ///
    /// # Arguments
    /// - `uuid` – The entity's UUID as a string.
    /// - `kind` – The entity type identifier (e.g., "minecraft:pig").
    /// - `name` – Optional custom name for the entity.
    ///
    /// # Returns
    /// A `HoverEvent::ShowEntity` variant containing the entity information.
    pub fn show_entity<P: Into<Cow<'static, str>>>(
        uuid: P,
        kind: P,
        name: Option<TextComponent>,
    ) -> Self {
        Self::ShowEntity {
            id: kind.into(),
            uuid: uuid.into(),
            name: match name {
                Some(name) => Some(name.0),
                None => None,
            },
        }
    }
}
