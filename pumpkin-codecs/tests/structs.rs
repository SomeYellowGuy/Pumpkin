use pumpkin_codecs::json_ops::JsonOps;
use pumpkin_codecs::{assert_decode, assert_decode_success, assert_encode_success, DataResult, Decode, DynamicOps, Lifecycle, MapLike};
use pumpkin_codecs_macros::{Decode, Encode};
use serde_json::json;
use pumpkin_codecs::codec::{FieldDecode, MapDecode};

#[derive(Debug, PartialEq, Eq, Clone, Encode)]
pub struct Book {
    name: String,
    author: String,
    pages: u32,
}

impl MapDecode for Book {
    fn map_decode<O: DynamicOps>(map: &impl MapLike<Value=O::Value>, ops: &'static O ) -> DataResult<Self> {
        let a0 = FieldDecode::decode_field::<O>("name", map, ops);
        let a1 = FieldDecode::decode_field::<O>("author", map, ops);
        let a2 = FieldDecode::decode_field::<O>("pages", map, ops);
        a0.apply_3(|name, author, pages| Self { name, author, pages }, a1, a2)
    }
}
impl Decode for Book {
    fn decode<O: DynamicOps>(input: O::Value, ops: &'static O) -> DataResult<(Self, O::Value)> {
        let map = DynamicOps::get_map(ops, &input);
        let single_result = DataResult::with_lifecycle(map, Lifecycle::Stable).flat_map(|map| { MapDecode::map_decode(map, ops) });
        DataResult::map(single_result, |s| (s, input))
    }
}

#[test]
fn unit() {
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Encode, Decode)]
    struct Unit;

    assert_encode_success!(Unit, JsonOps, json!({}));
    assert_decode_success!(Unit, json!({}), JsonOps, Unit);
    assert_decode!(Unit, json!(2), JsonOps, is_error);
    assert_decode!(Unit, json!("hello"), JsonOps, is_error);
}

#[test]
fn simple() {
    let object = Book {
        name: "Sample Book".to_string(),
        author: "Sample Author".to_string(),
        pages: 16,
    };

    assert_encode_success!(
        object,
        JsonOps,
        json!({
            "name": "Sample Book",
            "author": "Sample Author",
            "pages": 16,
        })
    );

    assert_decode_success!(
        Book,
        json!({
            "name": "The Great Gatsby",
            "author": "F. Scott Fitzgerald",
            "pages": 180
        }),
        JsonOps,
        Book {
            name: "The Great Gatsby".to_string(),
            author: "F. Scott Fitzgerald".to_string(),
            pages: 180
        }
    );

    assert_decode!(
        Book,
        json!({"name": "Untitled Book", "pages": 345}),
        JsonOps,
        is_error
    );

    assert_decode!(
        Book,
        json!({"name": "Untitled Book 2", "author": "Untitled Author", "pages": "98"}),
        JsonOps,
        is_error
    );
}

#[test]
fn composite() {
    #[derive(Debug, PartialEq, Eq, Clone, Encode, Decode)]
    pub struct Bookshelf {
        id: u32,
        // Optional, defaults to no books.
        #[codec(default)]
        books: Vec<Book>,
    }

    let example = Bookshelf {
        id: 1234,
        books: vec![
            Book {
                name: "Charlie and the Chocolate Factory".to_string(),
                author: "Roald Dahl".to_string(),
                pages: 192,
            },
            Book {
                name: "Infinibook".to_string(),
                author: "Infiniauthor".to_string(),
                pages: 1_000_000,
            },
        ],
    };

    assert_encode_success!(
        example,
        JsonOps,
        json!({
            "id": 1234,
            "books": [
                {
                    "name": "Charlie and the Chocolate Factory",
                    "author": "Roald Dahl",
                    "pages": 192
                },
                {
                    "name": "Infinibook",
                    "author": "Infiniauthor",
                    "pages": 1_000_000
                }
            ]
        })
    );

    let example = Bookshelf {
        id: 45193,
        // This example has no books, so no encoding of "books" will happen.
        books: vec![],
    };

    assert_encode_success!(example, JsonOps, json!({ "id": 45193 }));

    assert_decode_success!(
        Bookshelf,
        json!({ "id": 927 }),
        JsonOps,
        Bookshelf {
            id: 927,
            books: vec![]
        }
    );

    assert_decode!(
        Bookshelf,
        json!({"id": -1, "books": [
            {"name": "Book 1", "author": "Author 1", "pages": 100},
            {"name": "Book 2", "author": "Author 2", "pages": 200},
            {"name": "Book 3", "author": "Author 3", "pages": 300}
        ]}),
        JsonOps,
        is_error
    );
}
