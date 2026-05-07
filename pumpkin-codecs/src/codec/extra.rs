use uuid::Uuid;
use crate::{DataResult, Decode, DynamicOps, Encode, IntStream};
use crate::codec::list::validate_fixed_size;

/// Converts the first 4 elements of a [`Vec`] to an UUID.
pub fn uuid_from_vec(vec: Vec<i32>) -> Uuid {
    Uuid::from_u128(
        (vec[0] as u128) << 96 |
            (vec[1] as u128) << 64 |
            (vec[2] as u128) << 32 |
            (vec[3] as u128)
    )
}

// Replicates UUIDUtil.CODEC

impl Encode for Uuid {
    fn encode<O: DynamicOps>(&self, ops: &'static O, prefix: O::Value) -> DataResult<O::Value> {
        let bytes = self.as_u128();
        // First i32 is most significant.
        let ints = vec![
            (bytes >> 96 & 0xff) as i32,
            (bytes >> 64 & 0xff) as i32,
            (bytes >> 32 & 0xff) as i32,
            (bytes & 0xff) as i32
        ];
        IntStream(ints).encode(ops, prefix)
    }
}

impl Decode for Uuid {
    fn decode<O: DynamicOps>(input: O::Value, ops: &'static O) -> DataResult<(Self, O::Value)> {
        IntStream::decode(input, ops).flat_map(|(stream, p)| {
            let vec = stream.0;
            validate_fixed_size(vec, 4).map(|vec| (uuid_from_vec(vec), p))
        })
    }
}
