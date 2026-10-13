use pyo3::{
    exceptions::PyTypeError,
    prelude::*,
    types::{PyDict, PyString},
};

pub(crate) fn reject_unknown_kwargs(
    kwargs: Borrowed<'_, '_, PyDict>,
    allowed: &[&str],
) -> PyResult<()> {
    for (key, _) in kwargs.iter() {
        let key = key.cast::<PyString>()?;
        if !key.to_str().is_ok_and(|name| allowed.contains(&name)) {
            return Err(PyTypeError::new_err(format!(
                "unexpected keyword argument '{}'",
                key.to_string_lossy()
            )));
        }
    }
    Ok(())
}

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

derive_deftly::define_derive_deftly! {
    /// Derive keyword names and extraction from the parameter fields.
    /// Reject unknown keys when the type specifies `deny_unknown`.
    /// Marked fields are converted last, after other parameters are validated.
    ExtractOptions for struct:

    impl<$tgens> $ttype where $twheres {
        pub(crate) const FIELDS: &'static [&'static str] = &[
            ${for fields { ${concat $fname}, }}
        ];
    }

    impl<'__extract_a, '__extract_py, $tgens>
        pyo3::FromPyObject<'__extract_a, '__extract_py> for $ttype
        where $twheres
    {
        type Error = pyo3::PyErr;

        fn extract(ob: pyo3::Borrowed<'__extract_a, '__extract_py, pyo3::PyAny>) -> pyo3::PyResult<Self> {
            use pyo3::prelude::*;
            ${if tmeta(deny_unknown) {
                if let Ok(kwargs) = ob.cast::<pyo3::types::PyDict>() {
                    $crate::macros::reject_unknown_kwargs(kwargs, Self::FIELDS)?;
                }
            }}
            let mut params = Self::default();
            ${for fields {
                ${when fmeta(extract_last)}
                let ${paste pending_ $fname} =
                    $crate::macros::lookup(&ob, pyo3::intern!(ob.py(), ${concat $fname}));
            }}
            let mut remaining = ob.cast::<pyo3::types::PyDict>()
                .map_or(Self::FIELDS.len(), |dict| dict.len())
                ${for fields {
                    ${when fmeta(extract_last)}
                    .saturating_sub(usize::from(${paste pending_ $fname}.is_some()))
                }};
            ${for fields {
                ${when not(fmeta(extract_last))}
                if let Some(value) = (remaining > 0)
                    .then(|| $crate::macros::lookup(&ob, pyo3::intern!(ob.py(), ${concat $fname})))
                    .flatten()
                {
                    params.$fname = value.extract()?;
                    remaining -= 1;
                }
            }}
            ${for fields {
                ${when fmeta(extract_last)}
                if let Some(value) = ${paste pending_ $fname} {
                    params.$fname = value.extract()?;
                }
            }}
            // Avoid an unused-assignment warning after the last generated field.
            let _ = remaining;
            Ok(params)
        }
    }
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
