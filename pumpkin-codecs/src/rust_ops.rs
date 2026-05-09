use crate::Lifecycle;
use std::fmt::{Display, Formatter};
use std::vec::IntoIter;
use crate::{impl_struct_builder, impl_universal_struct_builder, DataResult, DynamicOps, MapLike, Number};
use crate::struct_builder::{ResultStructBuilder, StructBuilder, UniversalStructBuilder};

/// Represents a Rust value that can directly be encoded.
#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    None,
    Number(Number),
    String(String),

    List(Vec<Value>),
    ByteList(Vec<i8>),
    IntList(Vec<i32>),
    LongList(Vec<i64>),

    Map(Vec<(Value, Value)>),
}

fn display_list<T: Display>(f: &mut Formatter<'_>, list: &[T]) -> std::fmt::Result {
    let list = list.into_iter().map(|item| format!("{}", item)).collect::<Vec<String>>().join(", ");
    write!(f, "[{list}]")
}

fn display_map(f: &mut Formatter<'_>, map: &Vec<(Value, Value)>) -> std::fmt::Result {
    let map = map.into_iter().map(|(k, v)| format!("{}: {}", k, v)).collect::<Vec<String>>().join(", ");
    write!(f, "{{{map}}}")
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::None => write!(f, "None"),
            Value::Number(n) => write!(f, "{n}"),
            Value::String(s) => write!(f, "{s}"),
            Value::List(l) => display_list(f, l),
            Value::ByteList(l) => display_list(f, l),
            Value::IntList(l) => display_list(f, l),
            Value::LongList(l) => display_list(f, l),
            Value::Map(m) => display_map(f, m)
        }
    }
}

/// A [`DynamicOps`] to serialize to/deserialize from arbitrary Rust values.
pub struct RustOps;

impl DynamicOps for RustOps {
    type Value = Value;
    type StructBuilder = RustStructBuilder;

    fn empty(&self) -> Self::Value {
        Value::None
    }

    fn create_number(&self, n: Number) -> Self::Value {
        Value::Number(n)
    }

    fn create_string(&self, data: &str) -> Self::Value {
        Value::String(data.to_string())
    }

    fn create_list<I>(&self, values: I) -> Self::Value
    where
        I: IntoIterator<Item=Self::Value>
    {
        Value::List(values.into_iter().collect())
    }

    fn create_map<I>(&self, entries: I) -> Self::Value
    where
        I: IntoIterator<Item=(Self::Value, Self::Value)>
    {
        Value::Map(
            entries
                .into_iter()
                .collect(),
        )
    }

    fn get_number(&self, input: &Self::Value) -> DataResult<Number> {
        if let Value::Number(n) = input {
            DataResult::new_success(*n)
        } else {
            DataResult::new_error(format!("Not a number: {input}"))
        }
    }

    fn get_string(&self, input: &Self::Value) -> DataResult<String> {
        if let Value::String(n) = input {
            DataResult::new_success(n.clone())
        } else {
            DataResult::new_error(format!("Not a string: {input}"))
        }
    }

    fn get_map_iter<'a>(&'a self, input: &'a Self::Value) -> DataResult<impl Iterator<Item=(Self::Value, &'a Self::Value)> + 'a> {
        if let Value::Map(m) = input {
            DataResult::new_success(m.iter().map(|(k, v)| (k.clone(), v)))
        } else {
            DataResult::new_error(format!("Not a map: {input}"))
        }
    }

    fn get_map<'a>(&self, input: &'a Self::Value) -> DataResult<impl MapLike<Value=Self::Value> + 'a> {
        if let Value::Map(m) = input {
            DataResult::new_success(RustMapLike { map: m })
        } else {
            DataResult::new_error(format!("Not a map: {input}"))
        }
    }

    fn get_iter(&self, input: Self::Value) -> DataResult<impl Iterator<Item=Self::Value>> {
        match input {
            Value::List(v) => DataResult::new_success(RustListIter::List(v.into_iter())),

            Value::ByteList(v) => DataResult::new_success(RustListIter::ByteList(v.into_iter().map(|b| RustOps.create_byte(b)))),
            Value::IntList(v) => DataResult::new_success(RustListIter::IntList(v.into_iter().map(|i| RustOps.create_int(i)))),
            Value::LongList(v) => DataResult::new_success(RustListIter::LongList(v.into_iter().map(|l| RustOps.create_long(l)))),

            _ => DataResult::new_error(format!("Not a list: {input}"))
        }
    }

    fn merge_into_list(&self, list: Self::Value, value: Self::Value) -> DataResult<Self::Value> {
        match list {
            Value::None => DataResult::new_success(Value::List(vec![value])),

            Value::List(mut l) => {
                l.push(value);
                DataResult::new_success(Value::List(l))
            }
            Value::ByteList(l) => {
                let mut l: Vec<Value> = l.into_iter().map(|b| self.create_byte(b)).collect();
                l.push(value);
                DataResult::new_success(Value::List(l))
            }
            Value::IntList(l) => {
                let mut l: Vec<Value> = l.into_iter().map(|i| self.create_int(i)).collect();
                l.push(value);
                DataResult::new_success(Value::List(l))
            }
            Value::LongList(l) => {
                let mut l: Vec<Value> = l.into_iter().map(|l| self.create_long(l)).collect();
                l.push(value);
                DataResult::new_success(Value::List(l))
            }

            _ => DataResult::new_error(format!("Not a list: {list}"))
        }
    }

    fn merge_values_into_list<I>(&self, list: Self::Value, values: I) -> DataResult<Self::Value>
    where
        I: IntoIterator<Item=Self::Value>,
    {
        match list {
            Value::None => DataResult::new_success(Value::List(values.into_iter().collect())),

            Value::List(mut l) => {
                l.extend(values);
                DataResult::new_success(Value::List(l))
            }
            Value::ByteList(l) => {
                let mut l: Vec<Value> = l.into_iter().map(|b| self.create_byte(b)).collect();
                l.extend(values);
                DataResult::new_success(Value::List(l))
            }
            Value::IntList(l) => {
                let mut l: Vec<Value> = l.into_iter().map(|i| self.create_int(i)).collect();
                l.extend(values);
                DataResult::new_success(Value::List(l))
            }
            Value::LongList(l) => {
                let mut l: Vec<Value> = l.into_iter().map(|l| self.create_long(l)).collect();
                l.extend(values);
                DataResult::new_success(Value::List(l))
            }

            _ => DataResult::new_error(format!("Not a list: {list}"))
        }
    }

    fn merge_into_map(&self, map: Self::Value, key: Self::Value, value: Self::Value) -> DataResult<Self::Value>
    where
        Self::Value: Clone
    {
        match map {
            Value::None => DataResult::new_success(Value::Map(vec![(key, value)])),

            Value::Map(mut map) => {
                map.push((key, value));
                DataResult::new_success(Value::Map(map))
            }

            _ => DataResult::new_error(format!("Not a map: {map}"))
        }
    }

    fn merge_entries_into_map<I>(&self, map: Self::Value, entries: I) -> DataResult<Self::Value>
    where
        I: IntoIterator<Item=(Self::Value, Self::Value)>,
        Self::Value: Clone,
    {
        match map {
            Value::None => DataResult::new_success(Value::Map(entries.into_iter().collect())),

            Value::Map(mut map) => {
                map.extend(entries);
                DataResult::new_success(Value::Map(map))
            }

            _ => DataResult::new_error(format!("Not a map: {map}"))
        }
    }

    fn remove(&self, input: Self::Value, key: &str) -> Self::Value {
        if let Value::Map(mut m) = input {
            m = m.into_iter().filter(|(k, _)| k != &Value::String(key.to_string())).collect();
            Value::Map(m)
        } else {
            input
        }
    }

    fn convert_to<U>(&self, out_ops: &impl DynamicOps<Value=U>, input: Self::Value) -> U {
        match input {
            Value::None => out_ops.empty(),
            Value::Number(n) => out_ops.create_number(n),
            Value::String(s) => out_ops.create_string(&s),
            Value::List(_) => self.convert_list(out_ops, input),
            Value::ByteList(v) => out_ops.create_byte_list(v),
            Value::IntList(v) => out_ops.create_int_list(v),
            Value::LongList(v) => out_ops.create_long_list(v),
            Value::Map(_) => self.convert_map(out_ops, input)
        }
    }

    fn map_builder(&'static self) -> Self::StructBuilder {
        RustStructBuilder {
            builder: DataResult::new_success_with_lifecycle(
                Vec::new(),
                Lifecycle::Stable,
            ),
        }
    }
}

/// A single concrete type for an iterator of a Rust list object.
enum RustListIter {
    List(IntoIter<Value>),
    ByteList(std::iter::Map<IntoIter<i8>, fn(i8) -> Value>),
    IntList(std::iter::Map<IntoIter<i32>, fn(i32) -> Value>),
    LongList(std::iter::Map<IntoIter<i64>, fn(i64) -> Value>),
}

impl Iterator for RustListIter {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::List(iter) => iter.next(),
            Self::ByteList(iter) => iter.next(),
            Self::IntList(iter) => iter.next(),
            Self::LongList(iter) => iter.next(),
        }
    }
}

/// An implementation of [`MapLike`] for Rust objects.
/// The lifetime is that of the referenced map.
struct RustMapLike<'a> {
    map: &'a Vec<(Value, Value)>,
}

impl MapLike for RustMapLike<'_> {
    type Value = Value;

    fn get(&self, key: &Self::Value) -> Option<&Self::Value> {
        self.map.iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    fn get_str(&self, key: &str) -> Option<&Self::Value> {
        self.get(&Value::String(key.to_string()))
    }

    fn iter(&self) -> impl Iterator<Item = (Self::Value, &Self::Value)> + '_ {
        self.map.iter().map(|(k, v)| (k.clone(), v))
    }
}

impl Display for RustMapLike<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        display_map(f, self.map)
    }
}

/// An implementation of [`StructBuilder`] for Rust objects.
pub struct RustStructBuilder {
    builder: DataResult<Vec<(Value, Value)>>,
}

impl StructBuilder for RustStructBuilder {
    type Value = Value;

    impl_struct_builder!(builder);
    impl_universal_struct_builder!(builder, RustOps);
}

impl ResultStructBuilder for RustStructBuilder {
    type Result = Vec<(Value, Value)>;

    fn build_with_builder(
        self,
        builder: Self::Result,
        prefix: Self::Value,
    ) -> DataResult<Self::Value> {
        RustOps.merge_entries_into_map(prefix, builder)
    }
}

impl UniversalStructBuilder for RustStructBuilder {
    fn append(
        &self,
        key: Self::Value,
        value: Self::Value,
        mut builder: Self::Result,
    ) -> Self::Result {
        builder.push((key, value));
        builder
    }
}
