use pumpkin_codecs::codec::list::validate_fixed_size;
use pumpkin_codecs::{DataResult, Decode, DynamicOps, Encode};

/// A 4-dimensional vector with components of type `T`.
///
/// Currently, this only exists to provide serialization for colors.
#[derive(Clone, Copy, Debug, PartialEq, Hash, Eq, Default)]
pub struct Vector4<T> {
    /// The W component of the vector.
    pub w: T,
    /// The X component of the vector.
    pub x: T,
    /// The Y component of the vector.
    pub y: T,
    /// The Z component of the vector.
    pub z: T,
}

impl<T> Vector4<T> {
    /// Creates a new `Vector4` with the given components.
    ///
    /// # Arguments
    /// - `w` – The W component.
    /// - `x` – The X component.
    /// - `y` – The Y component.
    /// - `z` – The Z component.
    ///
    /// # Returns
    /// A new `Vector4` with the specified components.
    pub const fn new(w: T, x: T, y: T, z: T) -> Self {
        Self { w, x, y, z }
    }
}

impl Encode for Vector4<f32> {
    fn encode<O: DynamicOps>(&self, ops: &'static O, prefix: O::Value) -> DataResult<O::Value> {
        let list = vec![self.w, self.x, self.y, self.z];
        list.encode(ops, prefix)
    }
}

impl Decode for Vector4<f32> {
    fn decode<O: DynamicOps>(input: O::Value, ops: &'static O) -> DataResult<(Self, O::Value)> {
        Vec::<f32>::decode(input, ops)
            .flat_map(|(v, p)| validate_fixed_size(v, 4).map(|v| (v, p)))
            .map(|(v, p)| (Self::new(v[0], v[1], v[2], v[3]), p))
    }
}
