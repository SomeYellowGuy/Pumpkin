use std::borrow::Cow;

use super::{TextComponent, TextComponentBase};
use crate::uuid_util::LenientUuid;
use pumpkin_codecs::{DataResult, Decode, DynamicOps, Encode};
use pumpkin_codecs_macros::{Decode, Encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
        // TODO: If we change this to use Uuid, we can directly use this enum instead of also using CodecHoverEvent.
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
    ShowText {
        value: TextComponentBase,
    },
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
            Err(format!(
                "Value must be within range [1;99]: {}",
                count.unwrap()
            ))
        }
    }
}

impl From<CodecHoverEvent> for HoverEvent {
    fn from(value: CodecHoverEvent) -> Self {
        match value {
            CodecHoverEvent::ShowText { value } => Self::ShowText { value },
            CodecHoverEvent::ShowItem { id, count } => Self::ShowItem { id, count },
            CodecHoverEvent::ShowEntity { id, uuid, name } => Self::ShowEntity {
                id,
                uuid: uuid.0.to_string().into(),
                name,
            },
        }
    }
}

impl CodecHoverEvent {
    fn from_normal(value: &HoverEvent) -> Result<Self, uuid::Error> {
        match value {
            HoverEvent::ShowText { value } => Ok(CodecHoverEvent::ShowText {
                value: value.clone(),
            }),
            HoverEvent::ShowItem { id, count } => Ok(CodecHoverEvent::ShowItem {
                id: id.clone(),
                count: *count,
            }),
            HoverEvent::ShowEntity { id, uuid, name } => {
                // For now, we convert the Cow to a Uuid.
                Uuid::parse_str(&uuid).map(|uuid| CodecHoverEvent::ShowEntity {
                    id: id.clone(),
                    uuid: LenientUuid(uuid),
                    name: name.clone(),
                })
            }
        }
    }
}

impl Encode for HoverEvent {
    fn encode<O: DynamicOps>(&self, ops: &'static O, prefix: O::Value) -> DataResult<O::Value> {
        CodecHoverEvent::from_normal(self).map_or_else(
            |_| DataResult::new_error("Could not convert HoverEvent to a CodecHoverEvent"),
            |e| e.encode(ops, prefix),
        )
    }
}

impl Decode for HoverEvent {
    fn decode<O: DynamicOps>(input: O::Value, ops: &'static O) -> DataResult<(Self, O::Value)> {
        CodecHoverEvent::decode(input, ops).map(|(e, p)| (Self::from(e), p))
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
        Self::ShowText { value: text.0 }
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
