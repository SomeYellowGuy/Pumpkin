use crate::text::style::Style;
use crate::text::{TextComponent, TextComponentBase, TextContent};
use either::Either;
use pumpkin_codecs::codec::list::NonEmptyVec;
use pumpkin_codecs::codec::optional_field::OptionalFieldDecode;
use pumpkin_codecs::codec::{
    FieldDecode, FieldEncode, MapDecode, MapEncode, optional_field::OptionalFieldEncode,
};
use pumpkin_codecs::struct_builder::StructBuilder;
use pumpkin_codecs::{DataResult, Decode, DynamicOps, Encode, Lifecycle, MapLike};
use std::borrow::Cow;

impl TextComponentBase {
    fn inner_map_encode<O: DynamicOps, B: StructBuilder<Value = O::Value>>(
        &self,
        ops: &'static O,
        mut prefix: B,
    ) -> B {
        prefix = self.content.map_encode(ops, prefix);
        if !self.extra.is_empty() {
            // The "extra" tag is optional, but if it is present, it cannot be empty.
            prefix = self.extra.encode_field("extra", ops, prefix);
        }
        prefix = self.style.map_encode(ops, prefix);
        prefix
    }

    fn inner_map_decode<O: DynamicOps>(
        input: &impl MapLike<Value = O::Value>,
        ops: &'static O,
    ) -> DataResult<Self> {
        let content = TextContent::map_decode(input, ops);
        let extra = Option::<NonEmptyVec<Self>>::decode_optional_field("extra", input, ops, false)
            .map(|r| if let Some(vec) = r { vec.0 } else { Vec::new() });
        let style = Style::map_decode(input, ops);

        content.apply_3(
            |content, extra, style| Self {
                content: Box::new(content),
                extra,
                style: Box::new(style),
            },
            extra,
            style,
        )
    }

    fn inner_encode<O: DynamicOps>(
        &self,
        ops: &'static O,
        prefix: O::Value,
    ) -> DataResult<O::Value> {
        let mut builder = ops.map_builder();
        builder = self.inner_map_encode(ops, builder);
        builder.build(prefix)
    }

    fn inner_decode<O: DynamicOps>(
        input: O::Value,
        ops: &'static O,
    ) -> DataResult<(Self, O::Value)> {
        let map = ops.get_map(&input);
        let single_result = map
            .with_lifecycle(Lifecycle::Stable)
            .flat_map(|map| Self::inner_map_decode(&map, ops));
        single_result.map(|s| (s, input))
    }
}

#[allow(clippy::fallible_impl_from)]
impl From<NonEmptyVec<Self>> for TextComponentBase {
    fn from(value: NonEmptyVec<Self>) -> Self {
        // We make the first component the parent of the others.
        let mut bases_iter = value.vec().into_iter();
        // Since a `NonEmptyVec` is guaranteed to not be empty, it must
        // have a first element.
        let mut result = bases_iter.next().unwrap();
        for other in bases_iter {
            result.extra.push(other);
        }
        result
    }
}

struct InnerTextComponentBase(TextComponentBase);

impl Encode for InnerTextComponentBase {
    fn encode<O: DynamicOps>(&self, ops: &'static O, prefix: O::Value) -> DataResult<O::Value> {
        self.0.inner_encode(ops, prefix)
    }
}

impl Decode for InnerTextComponentBase {
    fn decode<O: DynamicOps>(input: O::Value, ops: &'static O) -> DataResult<(Self, O::Value)> {
        let base = TextComponentBase::inner_decode(input, ops);
        base.map(|(base, p)| (Self(base), p))
    }
}

type TextComponentEither =
    Either<Either<String, NonEmptyVec<TextComponentBase>>, InnerTextComponentBase>;

impl Encode for TextComponentBase {
    fn encode<O: DynamicOps>(&self, ops: &'static O, prefix: O::Value) -> DataResult<O::Value> {
        if let Some(text) = self.collapse_to_string() {
            text.encode(ops, prefix)
        } else {
            self.inner_encode(ops, prefix)
        }
    }
}

impl Decode for TextComponentBase {
    fn decode<O: DynamicOps>(input: O::Value, ops: &'static O) -> DataResult<(Self, O::Value)> {
        let either = TextComponentEither::decode(input, ops);
        either.map(|(e, p)| {
            let component = match e {
                Either::Left(e) => match e {
                    Either::Left(s) => TextComponent::text(s).0,
                    Either::Right(v) => v.into(),
                },
                Either::Right(b) => b.0,
            };
            (component, p)
        })
    }
}

impl Encode for TextComponent {
    fn encode<O: DynamicOps>(&self, ops: &'static O, prefix: O::Value) -> DataResult<O::Value> {
        self.0.encode(ops, prefix)
    }
}

impl Decode for TextComponent {
    fn decode<O: DynamicOps>(input: O::Value, ops: &'static O) -> DataResult<(Self, O::Value)> {
        TextComponentBase::decode(input, ops).map(|(base, p)| (Self(base), p))
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
                prefix = translate.encode_field("translatable", ops, prefix);
                prefix = fallback.encode_optional_field("fallback", ops, prefix);
                if !with.is_empty() {
                    prefix = with.encode_field("with", ops, prefix);
                }
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
            Self::Custom { .. } => {
                prefix = prefix
                    .with_errors_from(&DataResult::<()>::new_error("No matching codec found"));
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
                let translate = Cow::<'static, str>::decode_field("translatable", input, ops);
                let fallback = Option::<Cow<'static, str>>::decode_optional_field(
                    "fallback", input, ops, true,
                );
                // TODO: accept many other things as well here
                let with = Option::<Vec<TextComponentBase>>::decode_optional_field(
                    "with", input, ops, false,
                );
                translate.apply_3(
                    |translate, fallback, with| Self::Translate {
                        translate,
                        fallback,
                        with: with.unwrap_or_else(Vec::new),
                        bedrock_translate: None,
                    },
                    fallback,
                    with,
                )
            }
            TextContentType::EntityNames => {
                // TODO: Validate the selector.
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
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum TextContentType {
    Text,
    Translate,
    EntityNames,
    Keybind,
    // We don't serialize Custom.
}

impl TextContentType {
    /// All [`TextContent`] types.
    ///
    /// The order is important; it decides which contents are prioritized first.
    ///
    /// For example, trying to print `{"text: "foo", keybind: "bar"}` prints `"foo"` (`Text` is first in the priority).
    const ALL: [Self; 4] = [
        Self::Text,
        Self::Translate,
        Self::Keybind,
        Self::EntityNames,
    ];
}

// Fuzzy
impl TextContent {
    fn fuzzy_map_encode<O: DynamicOps, B: StructBuilder<Value = O::Value>>(
        &self,
        ops: &'static O,
        prefix: B,
    ) -> B {
        self.map_encode_specific(ops, prefix)
    }

    fn fuzzy_map_decode<O: DynamicOps>(
        input: &impl MapLike<Value = O::Value>,
        ops: &'static O,
    ) -> DataResult<Self> {
        for ty in TextContentType::ALL {
            let result = Self::map_decode_specific(ty, input, ops);
            if result.is_success() {
                return result;
            }
        }
        DataResult::new_error("No matching codec found")
    }
}

// Legacy
impl TextContent {
    fn legacy_map_decode<O: DynamicOps>(
        input: &impl MapLike<Value = O::Value>,
        ops: &'static O,
    ) -> DataResult<Self> {
        let ty = String::decode_field("type", input, ops);
        ty.flat_map(|s| match s.as_str() {
            "text" => Self::map_decode_specific(TextContentType::Text, input, ops),
            "translatable" => Self::map_decode_specific(TextContentType::Translate, input, ops),
            "selector" => Self::map_decode_specific(TextContentType::EntityNames, input, ops),
            "keybind" => Self::map_decode_specific(TextContentType::Keybind, input, ops),
            _ => DataResult::new_error(format!("Unknown element id: {s}")),
        })
    }
}

// Combined
impl MapEncode for TextContent {
    fn map_encode<O: DynamicOps, B: StructBuilder<Value = O::Value>>(
        &self,
        ops: &'static O,
        prefix: B,
    ) -> B {
        self.fuzzy_map_encode(ops, prefix)
    }
}

impl MapDecode for TextContent {
    fn map_decode<O: DynamicOps>(
        input: &impl MapLike<Value = O::Value>,
        ops: &'static O,
    ) -> DataResult<Self> {
        if input.get_str("type").is_some() {
            Self::legacy_map_decode(input, ops)
        } else {
            Self::fuzzy_map_decode(input, ops)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::text::click::ClickEvent;
    use crate::text::color::{ARGBColor, Color, NamedColor, RGBColor};
    use crate::text::hover::HoverEvent;
    use crate::text::style::Style;
    use crate::text::{TextComponent, TextComponentBase, TextContent};
    use crate::translation::Locale;
    use pumpkin_codecs::json_ops::JsonOps;
    use pumpkin_codecs::{
        assert_decode, assert_decode_success, assert_encode, assert_encode_success,
    };
    use serde_json::json;
    use uuid::Uuid;

    macro_rules! text_content_component {
        ($content:expr) => {
            TextComponent(TextComponentBase {
                content: Box::new($content),
                style: Box::new(Style::default()),
                extra: vec![],
            })
        };
    }

    #[test]
    fn text_contents_encode() {
        // This is simply encoded to a String because only plain text is present.
        assert_encode_success!(
            TextComponent::text("Hello world!"),
            JsonOps,
            json!("Hello world!")
        );
        assert_encode_success!(
            TextComponent::text("Hello world!").color(Color::Named(NamedColor::Blue)),
            JsonOps,
            json!({"text": "Hello world!", "color": "blue"})
        );

        assert_encode_success!(
            TextComponent::translate("foo", []),
            JsonOps,
            json!({"translatable": "foo"})
        );
        assert_encode_success!(
            TextComponent::translate("foo", [TextComponent::text("bar")]),
            JsonOps,
            json!({"translatable": "foo", "with": ["bar"]})
        );
        assert_encode_success!(
            TextComponent::translate(
                "foo",
                [TextComponent::text("bar").color(Color::Named(NamedColor::Red))]
            ),
            JsonOps,
            json!({"translatable": "foo", "with": [{"text": "bar", "color": "red"}]})
        );

        assert_encode_success!(
            text_content_component!(TextContent::EntityNames {
                selector: "@p".into(),
                separator: None
            }),
            JsonOps,
            json!({
                "selector": "@p"
            })
        );
        assert_encode_success!(
            text_content_component!(TextContent::EntityNames {
                selector: "@p".into(),
                separator: Some(",".into())
            }),
            JsonOps,
            json!({
                "selector": "@p",
                "separator": ","
            })
        );

        assert_encode_success!(
            text_content_component!(TextContent::Keybind {
                keybind: "key.forward".into()
            }),
            JsonOps,
            json!({
                "keybind": "key.forward"
            })
        );

        assert_encode!(
            text_content_component!(TextContent::Custom {
                key: Default::default(),
                locale: Locale::EnUs,
                with: vec![]
            }),
            JsonOps,
            is_error
        );
    }

    #[test]
    fn text_contents_decode() {
        assert_decode_success!(
            TextComponent,
            json!("baz"),
            JsonOps,
            TextComponent::text("baz")
        );
        assert_decode_success!(
            TextComponent,
            json!({"text": "baz", "bold": true}),
            JsonOps,
            TextComponent::text("baz").bold()
        );

        assert_decode_success!(
            TextComponent,
            json!({"translatable": "example", "with": ["123", "456"]}),
            JsonOps,
            TextComponent::translate(
                "example",
                [TextComponent::text("123"), TextComponent::text("456")]
            )
        );

        assert_decode_success!(
            TextComponent,
            json!({"selector": "@e[]"}),
            JsonOps,
            text_content_component!(TextContent::EntityNames {
                selector: "@e[]".into(),
                separator: None
            })
        );

        assert_decode_success!(
            TextComponent,
            json!({"keybind": "example"}),
            JsonOps,
            text_content_component!(TextContent::Keybind {
                keybind: "example".into()
            })
        );

        // Legacy component format
        assert_decode_success!(
            TextComponent,
            json!({"type": "text", "text": "baz"}),
            JsonOps,
            TextComponent::text("baz")
        );
        assert_decode_success!(
            TextComponent,
            json!({"type": "keybind", "keybind": "example"}),
            JsonOps,
            text_content_component!(TextContent::Keybind {
                keybind: "example".into()
            })
        );
        assert_decode!(
            TextComponent,
            json!({"type": "text", "keybind": "example"}),
            JsonOps,
            is_error
        );

        // Priority
        assert_decode_success!(
            TextComponent,
            json!({"text": "first", "keybind": "second"}),
            JsonOps,
            TextComponent::text("first")
        );
        assert_decode_success!(
            TextComponent,
            json!({"translatable": "first", "keybind": "second"}),
            JsonOps,
            TextComponent::translate("first", [])
        );
        assert_decode_success!(
            TextComponent,
            json!({"keybind": "first", "selector": "second"}),
            JsonOps,
            text_content_component!(TextContent::Keybind {
                keybind: "first".into()
            })
        );
    }

    #[test]
    fn style_color() {
        assert_encode_success!(
            TextComponent::text("a").color(Color::Named(NamedColor::Blue)),
            JsonOps,
            json!({"text": "a", "color": "blue"})
        );
        assert_encode_success!(
            TextComponent::text("b").color(Color::Rgb(RGBColor::new(127, 127, 255))),
            JsonOps,
            json!({"text": "b", "color": "#7F7FFF"})
        );
        assert_encode_success!(
            TextComponent::text("c").color(Color::Reset),
            JsonOps,
            json!({"text": "c", "color": "reset"})
        );

        assert_decode_success!(
            TextComponent,
            json!({"text": "a", "color": "light_purple"}),
            JsonOps,
            TextComponent::text("a").color(Color::Named(NamedColor::LightPurple))
        );
        assert_decode!(
            TextComponent,
            json!({"text": "a", "color": "orange"}),
            JsonOps,
            is_error
        );
        assert_decode_success!(
            TextComponent,
            json!({"text": "a", "color": "reset"}),
            JsonOps,
            TextComponent::text("a").color(Color::Reset)
        );

        assert_decode!(
            TextComponent,
            json!({"text": "a", "color": "#10101010"}),
            JsonOps,
            is_error
        );
        assert_decode_success!(
            TextComponent,
            json!({"text": "a", "color": "#101010"}),
            JsonOps,
            TextComponent::text("a").color(Color::Rgb(RGBColor::new(16, 16, 16)))
        );
        assert_decode_success!(
            TextComponent,
            json!({"text": "a", "color": "#1010"}),
            JsonOps,
            TextComponent::text("a").color(Color::Rgb(RGBColor::new(0, 16, 16)))
        );
        assert_decode_success!(
            TextComponent,
            json!({"text": "a", "color": "#10"}),
            JsonOps,
            TextComponent::text("a").color(Color::Rgb(RGBColor::new(0, 0, 16)))
        );
    }

    #[test]
    fn style_events() {
        // Click events
        assert_encode_success!(
            TextComponent::text("test").click_event(ClickEvent::SuggestCommand {
                command: "list".into()
            }),
            JsonOps,
            json!({"text": "test", "click_event": { "action": "suggest_command", "command": "list" }})
        );
        assert_encode_success!(
            TextComponent::text("test").click_event(ClickEvent::ChangePage { page: 42 }),
            JsonOps,
            json!({"text": "test", "click_event": { "action": "change_page", "page": 42 }})
        );

        assert_decode!(
            TextComponent,
            json!({"text": "test", "click_event": { "action": "run_command", "command": "list" }}),
            JsonOps,
            is_success
        );
        assert_decode!(
            TextComponent,
            json!({"text": "test", "click_event": { "action": "change_page", "page": 42 }}),
            JsonOps,
            is_success
        );
        assert_decode!(
            TextComponent,
            json!({"text": "test", "click_event": { "type": "change_page", "page": 42 }}),
            JsonOps,
            is_error
        );

        // Hover events
        assert_encode_success!(
            TextComponent::text("test").hover_event(HoverEvent::ShowText {
                value: TextComponent::text("cool tooltip").0
            }),
            JsonOps,
            json!({"text": "test", "hover_event": { "action": "show_text", "value": "cool tooltip" }})
        );
        assert_encode_success!(
            TextComponent::text("test").hover_event(HoverEvent::ShowEntity {
                id: "minecraft:skeleton".into(),
                uuid: Uuid::from_u64_pair(1234, 5678).to_string().into(),
                name: None
            }),
            JsonOps,
            json!({"text": "test", "hover_event": { "action": "show_entity", "id": "minecraft:skeleton", "uuid": [0, 1234, 0, 5678] }})
        );
        assert_encode_success!(
            TextComponent::text("test").hover_event(HoverEvent::ShowItem {
                id: "minecraft:stick".into(),
                count: 64
            }),
            JsonOps,
            json!({"text": "test", "hover_event": { "action": "show_item", "id": "minecraft:stick", "count": 64 }})
        );
        assert_encode_success!(
            TextComponent::text("test").hover_event(HoverEvent::ShowItem {
                id: "minecraft:apple".into(),
                count: 1
            }),
            JsonOps,
            json!({"text": "test", "hover_event": { "action": "show_item", "id": "minecraft:apple" }})
        );

        assert_decode!(TextComponent, json!({"text": "test", "hover_event": { "action": "show_text", "value": "b" }}), JsonOps, is_success);
        assert_decode!(TextComponent, json!({"text": "test", "hover_event": { "action": "show_entity", "id": "zombie", "uuid": "4df03ec2-4a10-11f1-a5a0-325096b39f47" }}), JsonOps, is_success);
        assert_decode!(TextComponent, json!({"text": "test", "hover_event": { "action": "show_item", "id": "acacia_boat" }}), JsonOps, is_success);
    }

    #[test]
    fn style_others() {
        assert_encode_success!(
            TextComponent::text("style").bold().italic().underlined().obfuscated().strikethrough(),
            JsonOps,
            json!({"text": "style", "bold": true, "underlined": true, "italic": true, "underlined": true, "strikethrough": true, "obfuscated": true }),
        );

        assert_encode_success!(
            TextComponent::text("style").shadow_color(ARGBColor::new(255, 255, 255, 255)),
            JsonOps,
            json!({"text": "style", "shadow_color": -1 }),
        );

        assert_decode_success!(
            TextComponent,
            json!({"text": "style", "bold": true, "underlined": true, "italic": true, "underlined": true, "strikethrough": true, "obfuscated": true }),
            JsonOps,
            TextComponent::text("style").bold().italic().underlined().obfuscated().strikethrough()
        );

        assert_decode_success!(
            TextComponent,
            json!({"text": "style", "shadow_color": -1 }),
            JsonOps,
            TextComponent::text("style").shadow_color(ARGBColor::new(255, 255, 255, 255))
        );
        assert_decode_success!(
            TextComponent,
            json!({"text": "style", "shadow_color": [1.0, 1.0, 1.0, 1.0] }),
            JsonOps,
            TextComponent::text("style").shadow_color(ARGBColor::new(255, 255, 255, 255))
        );
        assert_decode!(
            TextComponent,
            json!({"text": "style", "shadow_color": [1.0, 1.0, 1.0] }),
            JsonOps,
            is_error
        );
        assert_decode!(
            TextComponent,
            json!({"text": "style", "shadow_color": [1.0, 1.0, 1.0, 1.0, 1.0] }),
            JsonOps,
            is_error
        );
    }

    #[test]
    fn extra() {
        assert_encode_success!(
            TextComponent::text("Hello world!").add_text("Child component"),
            JsonOps,
            json!({"text": "Hello world!", "extra": ["Child component"]}),
        );

        assert_decode_success!(
            TextComponent,
            json!({"text": "Hello world!", "extra": ["Child component"]}),
            JsonOps,
            TextComponent::text("Hello world!").add_text("Child component")
        );

        assert_decode!(
            TextComponent,
            json!({"text": "Hello world!"}),
            JsonOps,
            is_success
        );
        assert_decode!(
            TextComponent,
            // An empty "extra" list is not allowed.
            json!({"text": "Hello world!", "extra": []}),
            JsonOps,
            is_error
        );

        assert_decode_success!(
            TextComponent,
            json!(["a", "b", "c"]),
            JsonOps,
            TextComponent::text("a").add_text("b").add_text("c")
        );
        assert_decode_success!(
            TextComponent,
            json!(["d", {"text": "e", "color": "#ffffff"}]),
            JsonOps,
            TextComponent::text("d").add_child(TextComponent::text("e").color(Color::Rgb(RGBColor::new(255, 255, 255)))),
        );
    }
}
