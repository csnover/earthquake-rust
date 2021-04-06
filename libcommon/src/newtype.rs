// TODO:
// All of this newtype stuff seems like it absolutely should be easier to do.
// 1. Sending types like `newtype_num!(@impl $ident, $ty)` does not work
//    because the type will not match the `$ty` token so every type must be
//    explicitly enumerated in the main `newtype_num` pattern.
// 2. Using generics for the conversion impls does not work because
//    specialization is not stable as of rustc 1.48.0, and core includes a
//    generic `impl <T> From<T> for T`, which conflicts with
//    `impl <T: std::convert::From<$ty>> From<T> for $ident`.
// 3. `derive_more::From` + `#[from(forward)]` (as of 0.99.11) does not work
//    correctly with `TryInto` for some reason.

#[macro_export]
macro_rules! newtype_num {
    // Convert from the other type infallibly into this type
    (@from $ident:ident, $($from_ty:ty)+) => {
        $(impl ::core::convert::From<$from_ty> for $ident {
            fn from(value: $from_ty) -> Self {
                Self(value.into())
            }
        })+
    };

    // Convert from this type infallibly into the other type
    (@into $ident:ident, $($into_ty:ty)+) => {
        $(impl ::core::convert::From<$ident> for $into_ty {
            fn from(value: $ident) -> Self {
                use $crate::convert::UnwrapFrom;
                // This is using try_from + unwrap to support isize/usize; see
                // https://github.com/rust-lang/rust/issues/70460
                <$into_ty>::unwrap_from(value.0)
            }
        })+
    };

    // Convert from the other type fallibly into this inner type
    (@try_from $ident:ident, $inner_ty:ty, $($try_ty:ty)+) => {
        $(impl ::core::convert::TryFrom<$try_ty> for $ident {
            type Error = <$inner_ty as ::core::convert::TryFrom<$try_ty>>::Error;
            fn try_from(value: $try_ty) -> ::core::result::Result<Self, Self::Error> {
                Ok(Self(::core::convert::TryFrom::try_from(value)?))
            }
        })+
    };

    // Convert from this type fallibly into the other type
    (@try_into $ident:ident, $inner_ty:ty, $($try_ty:ty)+) => {
        $(impl ::core::convert::TryFrom<$ident> for $try_ty {
            type Error = <$try_ty as ::core::convert::TryFrom<$inner_ty>>::Error;
            fn try_from(value: $ident) -> ::core::result::Result<Self, Self::Error> {
                ::core::convert::TryFrom::try_from(value.0)
            }
        })+
    };

    (@decl [$($meta:meta),*], $vis:vis, $ident:ident, $ty_vis:vis, $ty:ty) => {
        #[derive(
            ::core::clone::Clone,
            ::core::marker::Copy,
            ::core::default::Default,
            ::core::cmp::Eq, ::core::cmp::Ord, ::core::cmp::PartialEq, ::core::cmp::PartialOrd,
            ::derive_more::Display,
            ::derive_more::Binary, ::derive_more::Octal,
            ::derive_more::LowerHex, ::derive_more::UpperHex,
            ::derive_more::Add, ::derive_more::Sub,
            ::derive_more::BitAnd, ::derive_more::BitOr, ::derive_more::BitXor,
            ::derive_more::Mul, ::derive_more::Div, ::derive_more::Rem,
            ::derive_more::Shr, ::derive_more::Shl,
            ::derive_more::AddAssign, ::derive_more::SubAssign,
            ::derive_more::BitAndAssign, ::derive_more::BitOrAssign, ::derive_more::BitXorAssign,
            ::derive_more::MulAssign, ::derive_more::DivAssign, ::derive_more::RemAssign,
            ::derive_more::ShrAssign, ::derive_more::ShlAssign
        )]
        $(#[$meta])*
        #[mul(forward)]
        $vis struct $ident($ty_vis $ty);
    };

    (@impl $ident:ident, i8) => {
        $crate::newtype_num!(@from $ident, i8);
        $crate::newtype_num!(@into $ident, i8 i16 i32 i64 i128);
        $crate::newtype_num!(@try_from $ident, i8, u8 i16 u16 i32 u32 i64 u64 i128 u128);
        $crate::newtype_num!(@try_into $ident, i8, u8 u16 u32 u64 u128);
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@into $ident, isize);
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@try_from $ident, i8, isize usize);
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@try_into $ident, i8, usize);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@into $ident, isize);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@try_from $ident, i8, isize usize);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@try_into $ident, i8, usize);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@into $ident, isize);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@try_from $ident, i8, isize usize);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@try_into $ident, i8, usize);
    };
    (@impl $ident:ident, u8) => {
        $crate::newtype_num!(@from $ident, u8);
        $crate::newtype_num!(@into $ident, u8 i16 u16 i32 u32 i64 u64 i128 u128);
        $crate::newtype_num!(@try_from $ident, u8, i8 i16 u16 i32 u32 i64 u64 i128 u128);
        $crate::newtype_num!(@try_into $ident, u8, i8);
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@into $ident, isize usize);
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@try_from $ident, u8, isize usize);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@into $ident, isize usize);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@try_from $ident, u8, isize usize);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@into $ident, isize usize);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@try_from $ident, u8, isize usize);
    };
    (@impl $ident:ident, i16) => {
        $crate::newtype_num!(@from $ident, i8 u8 i16);
        $crate::newtype_num!(@into $ident, i16 i32 i64 i128);
        $crate::newtype_num!(@try_from $ident, i16, u16 i32 u32 i64 u64 i128 u128);
        $crate::newtype_num!(@try_into $ident, i16, i8 u8 u16 u32 u64 u128);
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@from $ident, isize);
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@into $ident, isize);
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@try_from $ident, i16, usize);
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@try_into $ident, i16, usize);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@into $ident, isize);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@try_from $ident, i16, isize usize);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@try_into $ident, i16, usize);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@into $ident, isize);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@try_from $ident, i16, isize usize);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@try_into $ident, i16, usize);
    };
    (@impl $ident:ident, u16) => {
        $crate::newtype_num!(@from $ident, u8 u16);
        $crate::newtype_num!(@into $ident, u16 i32 u32 i64 u64 i128 u128);
        $crate::newtype_num!(@try_from $ident, u16, i8 i16 i32 u32 i64 u64 i128 u128);
        $crate::newtype_num!(@try_into $ident, u16, i8 u8 i16);
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@from $ident, usize);
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@into $ident, usize);
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@try_from $ident, u16, isize);
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@try_into $ident, u16, isize);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@into $ident, isize usize);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@try_from $ident, u16, isize usize);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@into $ident, isize usize);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@try_from $ident, u16, isize usize);
    };
    (@impl $ident:ident, i32) => {
        $crate::newtype_num!(@from $ident, i8 u8 i16 u16 i32);
        $crate::newtype_num!(@into $ident, i32 i64 i128);
        $crate::newtype_num!(@try_from $ident, i32, u32 i64 u64 i128 u128);
        $crate::newtype_num!(@try_into $ident, i32, i8 u8 i16 u16 u32 u64 u128);
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@from $ident, isize usize);
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@try_into $ident, i32, isize usize);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@from $ident, isize);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@into $ident, isize);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@try_from $ident, i32, usize);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@try_into $ident, i32, usize);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@into $ident, isize);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@try_from $ident, i32, isize usize);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@try_into $ident, i32, usize);
    };
    (@impl $ident:ident, u32) => {
        $crate::newtype_num!(@from $ident, u8 u16 u32);
        $crate::newtype_num!(@into $ident, u32 i64 u64 i128 u128);
        $crate::newtype_num!(@try_from $ident, u32, i8 i16 i32 i64 u64 i128 u128);
        $crate::newtype_num!(@try_into $ident, u32, i8 u8 i16 u16 i32);
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@from $ident, usize);
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@try_from $ident, u32, isize);
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@try_into $ident, u32, isize usize);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@from $ident, usize);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@into $ident, usize);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@try_from $ident, u32, isize);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@try_into $ident, u32, isize);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@into $ident, isize usize);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@try_from $ident, u32, isize usize);
    };
    (@impl $ident:ident, i64) => {
        $crate::newtype_num!(@from $ident, i8 u8 i16 u16 i32 u32 i64);
        $crate::newtype_num!(@into $ident, i64 i128);
        $crate::newtype_num!(@try_from $ident, i64, u64 i128 u128);
        $crate::newtype_num!(@try_into $ident, i64, i8 u8 i16 u16 i32 u32 u64 u128);
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@from $ident, isize usize);
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@try_into $ident, i64, isize usize);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@from $ident, isize usize);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@try_into $ident, i64, isize usize);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@from $ident, isize);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@into $ident, isize);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@try_from $ident, i64, usize);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@try_into $ident, i64, usize);
    };
    (@impl $ident:ident, u64) => {
        $crate::newtype_num!(@from $ident, u8 u16 u32 u64);
        $crate::newtype_num!(@into $ident, u64 i128 u128);
        $crate::newtype_num!(@try_from $ident, u64, i8 i16 i32 i64 i128 u128);
        $crate::newtype_num!(@try_into $ident, u64, i8 u8 i16 u16 i32 u32 i64);
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@from $ident, usize);
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@try_from $ident, u64, isize);
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@try_into $ident, u64, isize usize);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@from $ident, usize);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@try_from $ident, u64, isize);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@try_into $ident, u64, isize usize);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@from $ident, usize);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@into $ident, usize);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@try_from $ident, u64, isize);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@try_into $ident, u64, isize);
    };
    (@impl $ident:ident, i128) => {
        $crate::newtype_num!(@from $ident, i8 u8 i16 u16 i32 u32 i64 u64 i128);
        $crate::newtype_num!(@into $ident, i128);
        $crate::newtype_num!(@try_from $ident, i128, u128);
        $crate::newtype_num!(@try_into $ident, i128, i8 u8 i16 u16 i32 u32 i64 u64 u128);
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@from $ident, isize usize);
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@try_into $ident, i128, isize usize);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@from $ident, isize usize);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@try_into $ident, i128, isize usize);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@from $ident, isize usize);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@try_into $ident, i128, isize usize);
    };
    (@impl $ident:ident, u128) => {
        $crate::newtype_num!(@from $ident, u8 u16 u32 u64 u128);
        $crate::newtype_num!(@into $ident, u128);
        $crate::newtype_num!(@try_from $ident, u128, i8 i16 i32 i64 i128);
        $crate::newtype_num!(@try_into $ident, u128, i8 u8 i16 u16 i32 u32 i64 u64 i128);
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@from $ident, usize);
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@try_from $ident, u128, isize);
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@try_into $ident, u128, isize usize);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@from $ident, usize);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@try_from $ident, u128, isize);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@try_into $ident, u128, isize usize);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@from $ident, usize);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@try_from $ident, u128, isize);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@try_into $ident, u128, isize usize);
    };

    (@impl $ident:ident, isize) => {
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@impl $ident, i16);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@impl $ident, i32);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@impl $ident, i64);
    };
    (@impl $ident:ident, usize) => {
        #[cfg(target_pointer_width = "16")]
        $crate::newtype_num!(@impl $ident, u16);
        #[cfg(target_pointer_width = "32")]
        $crate::newtype_num!(@impl $ident, u32);
        #[cfg(target_pointer_width = "64")]
        $crate::newtype_num!(@impl $ident, u64);
    };

    ($(#[$meta: meta])* $vis:vis struct $ident:ident($ty_vis:vis i8);) => {
        $crate::newtype_num!(@decl [$($meta),*], $vis, $ident, $ty_vis, i8);
        $crate::newtype_num!(@impl $ident, i8);
    };
    ($(#[$meta: meta])* $vis:vis struct $ident:ident($ty_vis:vis u8);) => {
        $crate::newtype_num!(@decl [$($meta),*], $vis, $ident, $ty_vis, u8);
        $crate::newtype_num!(@impl $ident, u8);
    };
    ($(#[$meta: meta])* $vis:vis struct $ident:ident($ty_vis:vis i16);) => {
        $crate::newtype_num!(@decl [$($meta),*], $vis, $ident, $ty_vis, i16);
        $crate::newtype_num!(@impl $ident, i16);
    };
    ($(#[$meta: meta])* $vis:vis struct $ident:ident($ty_vis:vis u16);) => {
        $crate::newtype_num!(@decl [$($meta),*], $vis, $ident, $ty_vis, u16);
        $crate::newtype_num!(@impl $ident, u16);
    };
    ($(#[$meta: meta])* $vis:vis struct $ident:ident($ty_vis:vis i32);) => {
        $crate::newtype_num!(@decl [$($meta),*], $vis, $ident, $ty_vis, i32);
        $crate::newtype_num!(@impl $ident, i32);
    };
    ($(#[$meta: meta])* $vis:vis struct $ident:ident($ty_vis:vis u32);) => {
        $crate::newtype_num!(@decl [$($meta),*], $vis, $ident, $ty_vis, u32);
        $crate::newtype_num!(@impl $ident, u32);
    };
    ($(#[$meta: meta])* $vis:vis struct $ident:ident($ty_vis:vis i64);) => {
        $crate::newtype_num!(@decl [$($meta),*], $vis, $ident, $ty_vis, i64);
        $crate::newtype_num!(@impl $ident, i64);
    };
    ($(#[$meta: meta])* $vis:vis struct $ident:ident($ty_vis:vis u64);) => {
        $crate::newtype_num!(@decl [$($meta),*], $vis, $ident, $ty_vis, u64);
        $crate::newtype_num!(@impl $ident, u64);
    };
    ($(#[$meta: meta])* $vis:vis struct $ident:ident($ty_vis:vis i128);) => {
        $crate::newtype_num!(@decl [$($meta),*], $vis, $ident, $ty_vis, i128);
        $crate::newtype_num!(@impl $ident, i128);
    };
    ($(#[$meta: meta])* $vis:vis struct $ident:ident($ty_vis:vis u128);) => {
        $crate::newtype_num!(@decl [$($meta),*], $vis, $ident, $ty_vis, u128);
        $crate::newtype_num!(@impl $ident, u128);
    };
    ($(#[$meta: meta])* $vis:vis struct $ident:ident($ty_vis:vis isize);) => {
        $crate::newtype_num!(@decl [$($meta),*], $vis, $ident, $ty_vis, isize);
        $crate::newtype_num!(@impl $ident, isize);
    };
    ($(#[$meta: meta])* $vis:vis struct $ident:ident($ty_vis:vis usize);) => {
        $crate::newtype_num!(@decl [$($meta),*], $vis, $ident, $ty_vis, usize);
        $crate::newtype_num!(@impl $ident, usize);
    };
}
