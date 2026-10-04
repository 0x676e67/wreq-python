use indexmap::IndexMap;
use pyo3::{FromPyObject, pybacked::PyBackedStr};
use serde::{
    Serialize, Serializer,
    ser::{SerializeMap, SerializeSeq},
};

/// Query or form parameters from a `dict`, or from a sequence of pairs that may repeat keys,
/// such as `[("tag", "rust"), ("tag", "http")]`.
#[derive(FromPyObject)]
pub enum Params {
    Map(IndexMap<PyBackedStr, ParamValue>),
    List(Vec<(PyBackedStr, ParamValue)>),
}

/// A scalar parameter value: `bool`, `int`, `float` or `str`, serialized as its text form
/// (`true`/`false` for booleans).
#[derive(FromPyObject)]
pub enum ParamValue {
    Boolean(bool),
    Number(isize),
    Float64(f64),
    String(PyBackedStr),
}

impl Serialize for ParamValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            ParamValue::String(s) => serializer.serialize_str(s.as_ref()),
            ParamValue::Number(n) => serializer.serialize_i64(*n as i64),
            ParamValue::Float64(f) => serializer.serialize_f64(*f),
            ParamValue::Boolean(b) => serializer.serialize_bool(*b),
        }
    }
}

impl Serialize for Params {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Params::Map(map) => {
                let mut map_serializer = serializer.serialize_map(Some(map.len()))?;
                for (key, value) in map {
                    map_serializer
                        .serialize_entry(<PyBackedStr as AsRef<str>>::as_ref(key), value)?;
                }
                map_serializer.end()
            }
            Params::List(vec) => {
                let mut seq_serializer = serializer.serialize_seq(Some(vec.len()))?;
                for (key, value) in vec {
                    seq_serializer
                        .serialize_element(&(<PyBackedStr as AsRef<str>>::as_ref(key), value))?;
                }
                seq_serializer.end()
            }
        }
    }
}
