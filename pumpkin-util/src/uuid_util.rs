use either::Either;
use pumpkin_codecs::codec::extra::uuid_from_vec;
use pumpkin_codecs::{DataResult, Decode, DynamicOps, Encode};
use uuid::Uuid;

/// Parses UUIDs similar to Java's `UUID.fromString` method.
#[inline]
#[must_use]
pub fn parse_uuid(uuid: &str) -> Option<Vec<i32>> {
    // We can't directly use the uuid crate to parse UUIDs, as it parses them
    // in a different way from Java.

    if uuid.len() > 36 {
        // UUID string is too large.
        return None;
    }

    // Split by hyphen. (5 segments)
    let mut parts = uuid.split('-');
    let mut parsed_parts: [i64; 5] = [0; 5];

    for part in &mut parsed_parts {
        // If a part is empty, the parsing functions will error anyway - this is what we want.
        *part = i64::from_str_radix(parts.next()?, 16).ok()?;
    }

    if parts.next().is_some() {
        // UUIDs must have exactly 5 parts.
        return None;
    }

    let bits = [
        (parsed_parts[0] & 0xFFFFFFFF) << 32
            | (parsed_parts[1] & 0xFFFF) << 16
            | (parsed_parts[2] & 0xFFFF),
        (parsed_parts[3] & 0xFFFF) << 48 | (parsed_parts[4] & 0xFFFFFFFFFFFF),
    ];

    Some(vec![
        (bits[0] >> 32) as i32,
        bits[0] as i32,
        (bits[1] >> 32) as i32,
        bits[1] as i32,
    ])
}

#[derive(Debug, Copy, Clone)]
pub struct StringUuid(pub Uuid);

// Replicates UUIDUtil.STRING_CODEC

impl Encode for StringUuid {
    fn encode<O: DynamicOps>(&self, ops: &'static O, prefix: O::Value) -> DataResult<O::Value> {
        self.0.to_string().encode(ops, prefix)
    }
}

impl Decode for StringUuid {
    fn decode<O: DynamicOps>(input: O::Value, ops: &'static O) -> DataResult<(Self, O::Value)> {
        String::decode(input, ops).flat_map(|(s, p)| {
            parse_uuid(&s).map_or_else(
                || DataResult::new_error(format!("Invalid UUID {s}")),
                |vec| DataResult::new_success((Self(uuid_from_vec(&vec)), p)),
            )
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct LenientUuid(pub Uuid);

impl Encode for LenientUuid {
    fn encode<O: DynamicOps>(&self, ops: &'static O, prefix: O::Value) -> DataResult<O::Value> {
        self.0.encode(ops, prefix)
    }
}

impl Decode for LenientUuid {
    fn decode<O: DynamicOps>(input: O::Value, ops: &'static O) -> DataResult<(Self, O::Value)> {
        let either = Either::<Uuid, StringUuid>::decode(input, ops);
        either.map(|(either, p)| {
            let uuid = either.either(|u| u, |s| s.0);
            (Self(uuid), p)
        })
    }
}

#[cfg(test)]
mod test {
    use crate::uuid_util::parse_uuid;

    #[test]
    fn parse_uuids() {
        assert_eq!(
            parse_uuid("3d569d3a-93ef-44a0-9f1c-f69db9d37a56"),
            Some(vec![1029086522, -1813035872, -1625491811, -1177322922])
        );
        assert_eq!(
            parse_uuid("3d53a-f-40-c-f69db9d37a56"),
            Some(vec![251194, 983104, 849565, -1177322922])
        );
        assert_eq!(parse_uuid("3d53a-f40-c-f69db9d37a56"), None);
        assert_eq!(
            parse_uuid("fffffffffffffff-0-0-0-0"),
            Some(vec![-1, 0, 0, 0])
        );
        assert_eq!(parse_uuid("ffffffffffffffff-0-0-0-0"), None);
        assert_eq!(
            parse_uuid("+1-+2-+3-+4-+5"),
            Some(vec![1, 131075, 262144, 5])
        );
        assert_eq!(
            parse_uuid("aaaaaaaaaaaaaaa-bbbbbbbbbbbbbb-c-d-e"),
            Some(vec![-1431655766, -1145372660, 851968, 14])
        );
    }
}
