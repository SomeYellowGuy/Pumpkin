use pumpkin_codecs::json_ops::JsonOps;
use pumpkin_codecs::{assert_decode, assert_decode_success, assert_encode, assert_encode_success};
use pumpkin_codecs_macros::{Decode, Encode};
use serde_json::json;

#[test]
fn flatten() {
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Encode, Decode)]
    pub struct Section {
        from: u64,
        to: u64,
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq, Encode, Decode)]
    pub enum TextChange {
        #[codec(tag = "font_size")]
        FontSize {
            #[codec(flatten)]
            at: Section,
            size: u32,
        },
        #[codec(tag = "italic")]
        Italic {
            #[codec(flatten)]
            at: Section,
            state: bool,
        },
        #[codec(tag = "delete")]
        Delete {
            #[codec(flatten)]
            at: Section,
        },
    }

    assert_encode_success!(
        TextChange::FontSize {
            at: Section { from: 7, to: 9 },
            size: 34
        },
        JsonOps,
        json!({
            "type": "font_size",
            "from": 7,
            "to": 9,
            "size": 34,
        })
    );

    assert_encode_success!(
        TextChange::Italic {
            at: Section { from: 3, to: 12 },
            state: false
        },
        JsonOps,
        json!({
            "type": "italic",
            "state": false,
            "from": 3,
            "to": 12,
        })
    );

    assert_decode_success!(
        TextChange,
        json!({
            "type": "delete",
            "from": 45,
            "to": 51
        }),
        JsonOps,
        TextChange::Delete {
            at: Section { from: 45, to: 51 }
        }
    );

    assert_decode!(
        TextChange,
        json!({
            "type": "color",
            "color": "#FF33FF",
            "from": 45,
            "to": 51
        }),
        JsonOps,
        is_error
    );
}

#[test]
fn transparent() {
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Encode, Decode)]
    #[codec(transparent)]
    struct BoolWrapper(bool);

    assert_encode_success!(BoolWrapper(true), JsonOps, json!(true));
    assert_encode_success!(BoolWrapper(false), JsonOps, json!(false));

    assert_decode_success!(BoolWrapper, json!(true), JsonOps, BoolWrapper(true));
    assert_decode_success!(BoolWrapper, json!(false), JsonOps, BoolWrapper(false));
}

#[test]
fn validate() {
    fn range(value: &i32) -> Result<(), &str> {
        if (&1..=&10).contains(&value) {
            Ok(())
        } else {
            Err("Value must be in the interval [1, 10]")
        }
    }

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct LimitedI32(#[codec(name = "value", default = 2, validate = range)] i32);

    assert_encode_success!(LimitedI32(1), JsonOps, json!({"value": 1}));
    assert_encode_success!(LimitedI32(2), JsonOps, json!({}));
    assert_encode_success!(LimitedI32(3), JsonOps, json!({"value": 3}));
    assert_encode!(LimitedI32(0), JsonOps, is_error);
    assert_encode!(LimitedI32(11), JsonOps, is_error);

    assert_decode_success!(LimitedI32, json!({"value": 5}), JsonOps, LimitedI32(5));
    assert_decode_success!(LimitedI32, json!({"value": 7}), JsonOps, LimitedI32(7));
    assert_decode_success!(LimitedI32, json!({"value": 4}), JsonOps, LimitedI32(4));

    assert_decode_success!(LimitedI32, json!({"value": 2}), JsonOps, LimitedI32(2));
    assert_decode_success!(LimitedI32, json!({}), JsonOps, LimitedI32(2));

    assert_decode!(LimitedI32, json!({"value": -1}), JsonOps, is_error);
    assert_decode!(LimitedI32, json!({"value": 13}), JsonOps, is_error);
}

#[test]
fn rename_all() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    // The tags for each variant are uppercase (except for green)
    #[codec(rename_all = "UPPERCASE")]
    enum RainbowColor {
        Red,
        Orange,
        Yellow,
        #[codec(tag = "green")]
        Green,
        Blue,
        Indigo,
        Violet
    }

    assert_encode_success!(RainbowColor::Red, JsonOps, json!("RED"));
    assert_encode_success!(RainbowColor::Yellow, JsonOps, json!("YELLOW"));
    assert_encode_success!(RainbowColor::Green, JsonOps, json!("green"));

    assert_decode_success!(RainbowColor, json!("INDIGO"), JsonOps, RainbowColor::Indigo);
    assert_decode_success!(RainbowColor, json!("BLUE"), JsonOps, RainbowColor::Blue);
    assert_decode_success!(RainbowColor, json!("green"), JsonOps, RainbowColor::Green);
    assert_decode!(RainbowColor, json!("GREEN"), JsonOps, is_error);
    assert_decode!(RainbowColor, json!("violet"), JsonOps, is_error);
}
