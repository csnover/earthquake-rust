//! Traits for conversions between types.

/// Simple and safe type conversions that may fail in a controlled way under
/// some circumstances, but shouldn’t. It is the reciprocal of [`UnwrapInto`].
///
/// This is a convenience trait for performing unwrapping conversion for values
/// that should normally always be convertible but which can’t be guaranteed to
/// be convertible at compile time.
pub trait UnwrapFrom<T>: Sized {
    /// Performs the conversion.
    ///
    /// # Panics
    ///
    /// Panics if the conversion fails.
    fn unwrap_from(value: T) -> Self;
}

impl <T, U> UnwrapFrom<U> for T
where
    T: core::convert::TryFrom<U>,
    T::Error: core::fmt::Debug,
{
    fn unwrap_from(value: U) -> Self {
        Self::try_from(value).unwrap()
    }
}

/// An attempted conversion that consumes `self`, which may or may not be
/// expensive.
///
/// This is a convenience trait for performing unwrapping conversion for values
/// that should normally always be convertible but which can’t be guaranteed to
/// be convertible at compile time.
pub trait UnwrapInto<T>: Sized {
    /// Performs the conversion.
    ///
    /// # Panics
    ///
    /// Panics if the conversion fails.
    fn unwrap_into(self) -> T;
}

impl <T, U> UnwrapInto<U> for T
where
    U: UnwrapFrom<T>
{
    fn unwrap_into(self) -> U {
        U::unwrap_from(self)
    }
}
