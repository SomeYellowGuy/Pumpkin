use pumpkin_codecs::json_ops::JsonOps;
use pumpkin_codecs::{assert_decode, assert_decode_success, assert_encode_success, Encode, Decode};
use pumpkin_codecs_macros::{Decode, Encode};
use serde_json::json;

#[test]
fn flatten() {
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Encode, Decode)]
    pub struct Section {
        from: u64,
        to: u64
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq, Encode, Decode)]
    pub enum TextChange {
        #[codec(tag = "font_size")]
        FontSize {
            #[codec(flatten)]
            at: Section,
            size: u32
        },
        #[codec(tag = "italic")]
        Italic {
            #[codec(flatten)]
            at: Section,
            state: bool
        },
        #[codec(tag = "delete")]
        Delete {
            #[codec(flatten)]
            at: Section
        }
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
        TextChange::Delete { at: Section { from: 45, to: 51 } }
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
fn validate() {
    fn range(value: &i32) -> Result<(), &str> {
        if value >= &1 && value <= &10 {
            Ok(())
        } else {
            Err("Value must be in the interval [1, 10]")
        }
    }

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct LimitedI32(
        #[codec(name = "value", default = 2, validate = range)] i32
    );

    assert_encode_success!(LimitedI32(1), JsonOps, json!({"value": 1}));
    assert_encode_success!(LimitedI32(2), JsonOps, json!({"value": 2}));
    assert_encode_success!(LimitedI32(3), JsonOps, json!({"value": 3}));
    assert_encode_success!(LimitedI32(11), JsonOps, json!({"value": 3}));
}
