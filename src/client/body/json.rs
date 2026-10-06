use std::fmt;

use indexmap::IndexMap;
use pyo3::{FromPyObject, prelude::*, pybacked::PyBackedStr};
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{Error, MapAccess, SeqAccess, Visitor},
};

/// Represents a JSON value for HTTP requests.
/// Supports objects, arrays, numbers, strings, booleans, and null.
#[derive(FromPyObject, IntoPyObject, Serialize)]
#[serde(untagged)]
pub enum Json {
    Object(IndexMap<JsonString, Json>),
    Boolean(bool),
    Number(isize),
    Float(f64),
    String(JsonString),
    Null(Option<isize>),
    Array(Vec<Json>),
}

/// A JSON string: borrowed from Python when serializing a request body, owned when
/// deserializing a response.
#[derive(IntoPyObject, PartialEq, Eq, Hash)]
pub enum JsonString {
    PyString(PyBackedStr),
    RustString(String),
}

/// Builds a [`Json`] in one pass; an untagged derive would buffer every value and then try
/// each variant in turn.
struct JsonVisitor;

// ===== impl JsonString =====

impl FromPyObject<'_, '_> for JsonString {
    type Error = PyErr;

    #[inline]
    fn extract(ob: Borrowed<PyAny>) -> PyResult<Self> {
        ob.extract().map(Self::PyString)
    }
}

impl Serialize for JsonString {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            JsonString::PyString(pb) => serializer.serialize_str(pb.as_ref()),
            JsonString::RustString(s) => serializer.serialize_str(s),
        }
    }
}

impl<'de> Deserialize<'de> for JsonString {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer).map(JsonString::RustString)
    }
}

// ===== impl Json =====

impl<'de> Deserialize<'de> for Json {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(JsonVisitor)
    }
}

// ===== impl JsonVisitor =====

impl<'de> Visitor<'de> for JsonVisitor {
    type Value = Json;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON value")
    }

    fn visit_bool<E: Error>(self, v: bool) -> Result<Json, E> {
        Ok(Json::Boolean(v))
    }

    // Integers outside `isize` become floats, as the untagged variant order did.
    fn visit_i64<E: Error>(self, v: i64) -> Result<Json, E> {
        Ok(isize::try_from(v).map_or(Json::Float(v as f64), Json::Number))
    }

    fn visit_u64<E: Error>(self, v: u64) -> Result<Json, E> {
        Ok(isize::try_from(v).map_or(Json::Float(v as f64), Json::Number))
    }

    fn visit_f64<E: Error>(self, v: f64) -> Result<Json, E> {
        Ok(Json::Float(v))
    }

    fn visit_str<E: Error>(self, v: &str) -> Result<Json, E> {
        Ok(Json::String(JsonString::RustString(v.to_owned())))
    }

    fn visit_string<E: Error>(self, v: String) -> Result<Json, E> {
        Ok(Json::String(JsonString::RustString(v)))
    }

    fn visit_unit<E: Error>(self) -> Result<Json, E> {
        Ok(Json::Null(None))
    }

    fn visit_none<E: Error>(self) -> Result<Json, E> {
        Ok(Json::Null(None))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Json, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = seq.next_element()? {
            values.push(value);
        }
        Ok(Json::Array(values))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Json, A::Error> {
        let mut values = IndexMap::new();
        while let Some((key, value)) = map.next_entry()? {
            values.insert(key, value);
        }
        Ok(Json::Object(values))
    }
}
