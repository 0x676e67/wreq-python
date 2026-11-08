use std::fmt;

use pyo3::{
    FromPyObject,
    prelude::*,
    pybacked::PyBackedStr,
    types::{PyBool, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple},
};
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{DeserializeSeed, Error, MapAccess, SeqAccess, Visitor},
    ser::SerializeMap,
};

/// Represents a JSON value for HTTP requests.
/// Supports objects, arrays, numbers, strings, booleans, and null.
#[derive(IntoPyObject, Serialize)]
#[serde(untagged)]
pub enum Json {
    Object(JsonObject),
    Boolean(bool),
    Number(isize),
    Float(f64),
    String(JsonString),
    Null(Option<isize>),
    Array(Vec<Json>),
}

/// A JSON string: borrowed from Python when serializing a request body, owned when
/// deserializing a response.
#[derive(IntoPyObject)]
pub enum JsonString {
    PyString(PyBackedStr),
    RustString(String),
}

pub struct JsonObject(Vec<(JsonString, Json)>);

/// Builds a [`Json`] in one pass; an untagged derive would buffer every value and then try
/// each variant in turn.
struct JsonVisitor<'a>(&'a mut Vec<(JsonString, Json)>);

// ===== impl JsonObject =====

impl FromPyObject<'_, '_> for JsonObject {
    type Error = PyErr;

    fn extract(ob: Borrowed<PyAny>) -> PyResult<Self> {
        let dict = ob.cast::<PyDict>()?;
        let mut members = Vec::with_capacity(dict.len());
        for (k, v) in dict.iter() {
            members.push((k.extract()?, v.extract()?));
        }
        Ok(JsonObject(members))
    }
}

impl<'py> IntoPyObject<'py> for JsonObject {
    type Target = PyDict;
    type Output = Bound<'py, PyDict>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> PyResult<Self::Output> {
        let dict = PyDict::new(py);
        for (k, v) in self.0 {
            dict.set_item(k, v)?;
        }
        Ok(dict)
    }
}

impl Serialize for JsonObject {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

// ===== impl JsonString =====

impl FromPyObject<'_, '_> for Json {
    type Error = PyErr;

    fn extract(ob: Borrowed<PyAny>) -> PyResult<Self> {
        if ob.is_instance_of::<PyString>() {
            return ob.extract().map(Json::String);
        }
        if ob.is_instance_of::<PyBool>() {
            return ob.extract().map(Json::Boolean);
        }
        if ob.is_instance_of::<PyInt>() {
            return ob
                .extract()
                .map(Json::Number)
                .or_else(|_| ob.extract().map(Json::Float));
        }
        if ob.is_instance_of::<PyFloat>() {
            return ob.extract().map(Json::Float);
        }
        if ob.is_none() {
            return Ok(Json::Null(None));
        }
        if ob.is_instance_of::<PyDict>() {
            return ob.extract().map(Json::Object);
        }
        if ob.is_instance_of::<PyList>() || ob.is_instance_of::<PyTuple>() {
            return ob.extract().map(Json::Array);
        }
        ob.extract()
            .map(Json::Object)
            .or_else(|_| ob.extract().map(Json::Boolean))
            .or_else(|_| ob.extract().map(Json::Number))
            .or_else(|_| ob.extract().map(Json::Float))
            .or_else(|_| ob.extract().map(Json::String))
            .or_else(|_| ob.extract().map(Json::Array))
    }
}

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
        deserializer.deserialize_any(JsonVisitor(&mut Vec::new()))
    }
}

// ===== impl JsonVisitor =====

impl<'de> DeserializeSeed<'de> for JsonVisitor<'_> {
    type Value = Json;

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<Json, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for JsonVisitor<'_> {
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
        while let Some(value) = seq.next_element_seed(JsonVisitor(&mut *self.0))? {
            values.push(value);
        }
        Ok(Json::Array(values))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Json, A::Error> {
        let stack = self.0;
        let start = stack.len();
        while let Some(key) = map.next_key()? {
            let value = map.next_value_seed(JsonVisitor(&mut *stack))?;
            stack.push((key, value));
        }
        Ok(Json::Object(JsonObject(stack.drain(start..).collect())))
    }
}
