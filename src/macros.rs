use pyo3::{
    prelude::*,
    types::{PyDict, PyString},
};

/// Look up an optional keyword; a dict answers a missing key without raising KeyError.
pub(crate) fn lookup<'py>(
    ob: &Bound<'py, PyAny>,
    key: &Bound<'py, PyString>,
) -> Option<Bound<'py, PyAny>> {
    match ob.cast::<PyDict>() {
        Ok(dict) => dict.get_item(key).ok().flatten(),
        Err(_) => ob.get_item(key).ok(),
    }
}

/// Keys left to look up: a dict's length, or unbounded for other mappings.
pub(crate) fn remaining(ob: &Bound<'_, PyAny>) -> usize {
    ob.cast::<PyDict>().map_or(usize::MAX, |dict| dict.len())
}

macro_rules! extract_option {
    ($ob:expr, $params:expr, $field:ident) => {
        if let Some(value) =
            $crate::macros::lookup(&$ob, pyo3::intern!($ob.py(), stringify!($field)))
        {
            $params.$field = value.extract()?;
        }
    };
}

/// Like [`extract_option!`] for several fields, but stop looking up keys once every key
/// of a dict has been found, counting `$found` keys the caller already looked up. Unknown
/// keys are never found, so they make the scan run to the end and stay ignored.
macro_rules! extract_options {
    ($ob:expr, $params:expr, $found:expr, [$($field:ident),* $(,)?]) => {{
        let mut remaining = $crate::macros::remaining(&$ob).saturating_sub($found);
        $(
            if remaining > 0
                && let Some(value) =
                    $crate::macros::lookup(&$ob, pyo3::intern!($ob.py(), stringify!($field)))
            {
                $params.$field = value.extract()?;
                remaining -= 1;
            }
        )*
    }};
}

macro_rules! apply_option {
    (set_if_some, $builder:expr, $option:expr, $method:ident) => {
        if let Some(value) = $option.take() {
            $builder = $builder.$method(value);
        }
    };
    (set_if_some_ref, $builder:expr, $option:expr, $method:ident) => {
        if let Some(value) = $option.take() {
            $builder = $builder.$method(&value);
        }
    };
    (set_if_some_inner, $builder:expr, $option:expr, $method:ident) => {
        if let Some(value) = $option.take() {
            $builder = $builder.$method(value.0);
        }
    };
    (set_if_some_map, $builder:expr, $option:expr, $method:ident, $transform:expr) => {
        if let Some(value) = $option.take() {
            $builder = $builder.$method($transform(value));
        }
    };
    (set_if_some_map_ref, $builder:expr, $option:expr, $method:ident, $transform:expr) => {
        if let Some(value) = $option.take() {
            $builder = $builder.$method($transform(&value));
        }
    };
    (set_if_some_map_try, $builder:expr, $option:expr, $method:ident, $transform:expr) => {
        if let Some(value) = $option.take() {
            $builder = $builder.$method($transform(value)?);
        }
    };
    (set_if_true, $builder:expr, $option:expr, $method:ident, $default:expr) => {
        if $option.unwrap_or($default) {
            $builder = $builder.$method();
        }
    };
    (set_if_some_tuple, $builder:expr, $option:expr, $method:ident) => {
        if let Some(value) = $option.take() {
            $builder = $builder.$method(value.0, value.1);
        }
    };
    (set_if_some_tuple_inner, $builder:expr, $option:expr, $method:ident) => {
        if let Some(value) = $option.take() {
            $builder = $builder.$method(value.0.0, value.0.1);
        }
    };
    (set_if_some_iter_inner, $builder:expr, $option:expr, $method:ident) => {
        if let Some(value) = $option.take() {
            for item in value {
                $builder = $builder.$method(item.0);
            }
        }
    };
    (set_if_some_iter_inner_with_key, $builder:expr, $option:expr, $method:ident, $key:ident) => {
        if let Some(value) = $option.take() {
            for item in value.0 {
                $builder = $builder.$method($key, item);
            }
        }
    };
}

#[allow(unused_macro_rules)]
macro_rules! define_enum {
    ($(#[$meta:meta])* struct $struct_type:ident, $enum_type:ident, $ffi_type:ty, $($variant:ident),* $(,)?) => {
        define_enum!($(#[$meta])* struct $struct_type, $enum_type, $ffi_type, $( ($variant, $variant) ),*);
    };

    ($(#[$meta:meta])* const, struct $struct_type:ident, $enum_type:ident, $ffi_type:ty, $($variant:ident),* $(,)?) => {
        define_enum!($(#[$meta])* const, struct $struct_type, $enum_type, $ffi_type, $( ($variant, $variant) ),*);
    };

    ($(#[$meta:meta])* $enum_type:ident, $ffi_type:ty, $($variant:ident),* $(,)?) => {
        define_enum!($(#[$meta])* $enum_type, $ffi_type, $( ($variant, $variant) ),*);
    };

    ($(#[$meta:meta])* const, $enum_type:ident, $ffi_type:ty, $($variant:ident),* $(,)?) => {
        define_enum!($(#[$meta])* const, $enum_type, $ffi_type, $( ($variant, $variant) ),*);
    };

    ($(#[$meta:meta])* struct $struct_type:ident, $enum_type:ident, $ffi_type:ty, $(($rust_variant:ident, $ffi_variant:ident)),* $(,)?) => {
        define_enum!($(#[$meta])* $enum_type, $ffi_type, $(($rust_variant, $ffi_variant)),*);

        #[pymethods]
        #[allow(non_upper_case_globals)]
        impl $struct_type {
            $(
                #[classattr]
                const $rust_variant: $enum_type = $enum_type::$rust_variant;
            )*
        }
    };

    ($(#[$meta:meta])* const, struct $struct_type:ident, $enum_type:ident, $ffi_type:ty, $(($rust_variant:ident, $ffi_variant:ident)),* $(,)?) => {
        define_enum!($(#[$meta])* const, $enum_type, $ffi_type, $(($rust_variant, $ffi_variant)),*);

        #[pymethods]
        #[allow(non_upper_case_globals)]
        impl $struct_type {
            $(
                #[classattr]
                const $rust_variant: $enum_type = $enum_type::$rust_variant;
            )*
        }
    };

    ($(#[$meta:meta])* $enum_type:ident, $ffi_type:ty, $(($rust_variant:ident, $ffi_variant:ident)),* $(,)?) => {
        $(#[$meta])*
        #[pyclass(eq, eq_int, frozen, from_py_object)]
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        #[allow(non_camel_case_types)]
        #[allow(clippy::upper_case_acronyms)]
        pub enum $enum_type {
            $($rust_variant),*
        }

        impl $enum_type {
            pub fn into_ffi(self) -> $ffi_type {
                match self {
                    $(<$enum_type>::$rust_variant => <$ffi_type>::$ffi_variant,)*
                }
            }
        }
    };

    ($(#[$meta:meta])* const, $enum_type:ident, $ffi_type:ty, $(($rust_variant:ident, $ffi_variant:ident)),* $(,)?) => {
        $(#[$meta])*
        #[pyclass(eq, eq_int, from_py_object)]
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        #[allow(non_camel_case_types)]
        #[allow(clippy::upper_case_acronyms)]
        pub enum $enum_type {
            $($rust_variant),*
        }

        impl $enum_type {
            #[allow(dead_code)]
            pub const fn into_ffi(self) -> $ffi_type {
                match self {
                    $(<$enum_type>::$rust_variant => <$ffi_type>::$ffi_variant,)*
                }
            }

            #[allow(dead_code)]
            pub const fn from_ffi(ffi: $ffi_type) -> Self {
                #[allow(unreachable_patterns)]
                match ffi {
                    $(<$ffi_type>::$ffi_variant => <$enum_type>::$rust_variant,)*
                    _ => unreachable!(),
                }
            }
        }
    };
}

macro_rules! impl_print_str {
    (Debug, $typed:ident) => {
        impl std::fmt::Display for $typed {
            #[inline]
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Debug::fmt(&self.0, f)
            }
        }
    };
    (Display, $typed:ident) => {
        impl std::fmt::Display for $typed {
            #[inline]
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}
