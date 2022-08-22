//! Various macros.

/// Create a [`Default`] impl from `Self::default()`.  Its first argument is the
/// type name, and second argument is a constantly-evaluable expression, where
/// `Self` refers to the struct name.  If the expression is not
/// constantly-evaluable, prepend `!` before the struct name.
///
/// ```rust
/// pub struct S(usize);
/// qrshare_lib::default!(S = Self(0));
///
/// const S0: S = S::default();
/// assert_eq!(S0.0, 0);
/// ```
///
/// ```compile_fail
/// pub struct RuntimeDefault;
/// qrshare_lib::default!(!RuntimeDefault = Self);
/// const _: RuntimeDefault = RuntimeDefault::default();
/// ```
///
/// One can also add metadata (such as docstring or `cfg` statements) to the
/// generated `Self::default()` method using the following syntax.
///
/// ```rust
/// pub struct LongStructName(usize, usize, usize, usize);
/// qrshare_lib::default!(
///     /// Return the constant default value.
///     #[cfg(not(debug_assertions))] // only generate in release builds
///     LongStructName = Self(42, 42, 43, 54)
/// );
/// ```
#[macro_export]
macro_rules! default {
    // public interface
    ($(#[$m:meta])* $s:ident = $default:expr $(,)?) => {
        $crate::default_internal!($(#[$m])*, const $s,$default);
        $crate::default_internal!(impl Default $s);
    };
    ($(#[$m:meta])* ! $s:ident = $default:expr $(,)?) => {
        $crate::default_internal!($(#[$m])*, $s, $default);
        $crate::default_internal!(impl Default $s);
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! default_internal {
    (impl Default $s:ident) => {
        impl Default for $s {
            #[inline] fn default() -> Self { Self::default() }
        }
    };
    ($(#[$m:meta])*, const $s:ident, $default:expr) => {
        impl $s {
            $(#[$m])* pub const fn default() -> Self { $default }
        }
    };
    ($(#[$m:meta])*, $s:ident, $default:expr) => {
        impl $s {
            $(#[$m])* pub fn default() -> Self { $default }
        }
    };
}

/// A getter for a field of a brace-style struct with the following
/// requirements:
///
/// 1. (Skipped, waiting for `Option::unwrap_or` to become const.) The field has
/// type `F`, with a const method `F::unwrap_or(&self) -> T`.
/// 2. (In effect until `Option::unwrap_or` becomes const.) The field must have
/// `Option<T>` type.
/// 3. The aforementioned type `T` must implement `Copy`.
///
/// ```rust
/// struct Thing { field: Option<u8>, other: bool }
/// qrshare_lib::unwrap_getter!(Thing::field: u8 = 3);
///
/// assert_eq!(Thing { field: Some(2), other: false }.field(), 2);
/// assert_eq!(Thing { field: None, other: true }.field(), 3);
/// ```
#[macro_export]
macro_rules! unwrap_getter {
    ($(#[$m:meta])* $s:ident :: $f:ident : $ft:ty) => {
        $crate::unwrap_getter!($(#[$m])* $s :: $f : $ft = <$ft>::default());
    };
    ($(#[$m:meta])* $s:ident :: $f:ident : $ft:ty = $default:expr) => {
        impl $s where $ft: Copy {
            #[doc = concat!(
                "Get the field `", stringify!($s), ".", stringify!($f),
                "`, defaulting to the result of `", stringify!($default), "`",
            )]
            $(#[$m])* pub const fn $f(&self) -> $ft {
                match self.$f {
                    Some(f) => f,
                    None => $default,
                }
            }
        }
    };
}

pub struct Thing {
    field: Option<u8>,
}
unwrap_getter!(Thing::field: u8 = 3);
