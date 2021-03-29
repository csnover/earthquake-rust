use binrw::BinRead;
use derive_more::{Deref, DerefMut};
use std::{convert::TryInto, io::SeekFrom, marker::PhantomData};

#[doc(hidden)]
pub mod __private {
    pub use {binrw, bitflags::bitflags, derive_more::*, paste::paste};
}

/// A wrapper type for reading a `BitFlags` object from data with a different
/// underlying size.
#[derive(Copy, Clone, Deref, DerefMut)]
pub struct FlagsFrom<Flags, FromType>
where
    Flags: BitFlags,
{
    #[deref]
    #[deref_mut]
    inner: Flags,
    _phantom: PhantomData<FromType>,
}

impl <Flags: BitFlags + Default, FromType> Default for FlagsFrom<Flags, FromType> {
    fn default() -> Self {
        Self { inner: Flags::default(), _phantom: PhantomData }
    }
}

impl <Flags: BitFlags, FromType> From<Flags> for FlagsFrom<Flags, FromType> {
    fn from(inner: Flags) -> Self {
        Self { inner, _phantom: PhantomData }
    }
}

impl <Flags: BitFlags, FromType> core::fmt::Debug for FlagsFrom<Flags, FromType> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl <Flags, FromType> BinRead for FlagsFrom<Flags, FromType>
where
    Flags: BitFlags + 'static,
    FromType: TryInto<Flags::Bits> + BinRead + std::fmt::LowerHex + Copy,
{
    type Args = FromType::Args;

    fn read_options<R: binrw::io::Read + binrw::io::Seek>(reader: &mut R, options: &binrw::ReadOptions, args: Self::Args) -> binrw::BinResult<Self> {
        macro_rules! make_err {
            ($msg: literal, $flags: ty, $pos: expr, $value: ident) => {
                binrw::Error::AssertFail {
                    pos: $pos,
                    message: format!(concat!($msg, " {} flags 0x{:x}"), core::any::type_name::<$flags>(), $value),
                }
            }
        }

        let pos = reader.seek(SeekFrom::Current(0))?;
        let raw_value = FromType::read_options(reader, options, args)?;
        let bits = raw_value.try_into().map_err(|_| make_err!("Out of range value for", Flags, pos, raw_value))?;
        let flags = Flags::from_bits(bits).ok_or_else(|| make_err!("Invalid", Flags, pos, raw_value))?;
        Ok(Self { inner: flags, _phantom: PhantomData })
    }
}

/// A temporary wrapper trait for `BitFlags` implementations until
/// <https://github.com/bitflags/bitflags/issues/154> is fixed
pub trait BitFlags:
    core::iter::Extend<Self>
    + core::iter::FromIterator<Self>
    + core::ops::BitAnd<Output = Self>
    + core::ops::BitAndAssign
    + core::ops::BitOr<Output = Self>
    + core::ops::BitOrAssign
    + core::ops::BitXor<Output = Self>
    + core::ops::BitXorAssign
    + core::ops::Not<Output = Self>
    + core::ops::Sub<Output = Self>
    + core::ops::SubAssign
    + Copy
    + Clone
    + core::fmt::Debug
    + Eq
    + core::hash::Hash
    + Ord
    + PartialEq
    + PartialOrd
    + Sized
{
    type Bits;

    fn empty() -> Self;
    fn all() -> Self;
    fn bits(&self) -> Self::Bits;
    fn from_bits(bits: Self::Bits) -> Option<Self>;
    #[allow(clippy::missing_safety_doc)]
    unsafe fn from_bits_truncate(bits: Self::Bits) -> Self;
    #[allow(clippy::missing_safety_doc)]
    unsafe fn from_bits_unchecked(bits: Self::Bits) -> Self;
    fn is_empty(&self) -> bool;
    fn is_all(&self) -> bool;
    fn intersects(&self, other: Self) -> bool;
    fn contains(&self, other: Self) -> bool;
    fn insert(&mut self, other: Self);
    fn remove(&mut self, other: Self);
    fn toggle(&mut self, other: Self);
    fn set(&mut self, other: Self, value: bool);
}

#[macro_export]
/// Generates a C-style bitmask struct which can be used as a generic (via the
/// `BitFlags` trait) and with a `BinRead` implementation.
macro_rules! bitflags {
    (
        $(#[$outer:meta])*
        $vis:vis struct $BitFlags:ident: $T:ty {
            $($(#[$inner:meta])* const $field:ident = $value:expr;)*
        }
    ) => {
        $crate::bitflags::__private::paste! {
            $crate::bitflags::__private::bitflags! {
                $(#[$outer])*
                struct [< $BitFlags Inner >]: $T {
                    $(const $field = $value;)*
                }
            }

            $(#[$outer])*
            #[derive(
                Copy,
                Clone,
                Eq,
                Hash,
                Ord,
                PartialEq,
                PartialOrd
            )]
            #[derive(
                $crate::bitflags::__private::BitAnd,
                $crate::bitflags::__private::BitAndAssign,
                $crate::bitflags::__private::BitOr,
                $crate::bitflags::__private::BitOrAssign,
                $crate::bitflags::__private::BitXor,
                $crate::bitflags::__private::BitXorAssign,
                $crate::bitflags::__private::Not,
                $crate::bitflags::__private::Sub,
                $crate::bitflags::__private::SubAssign,
            )]
            $vis struct $BitFlags {
                inner: [< $BitFlags Inner >],
            }

            impl $BitFlags {
                $(
                    $(#[$inner])*
                    $vis const $field: $BitFlags = Self { inner: [< $BitFlags Inner >]::$field };
                )*
            }

            impl $crate::bitflags::BitFlags for $BitFlags {
                type Bits = $T;

                fn empty() -> Self { Self { inner: [< $BitFlags Inner >]::empty() } }
                fn all() -> Self { Self { inner: [< $BitFlags Inner >]::all() } }
                fn bits(&self) -> Self::Bits { self.inner.bits() }
                fn from_bits(bits: Self::Bits) -> Option<Self> { Some(Self { inner: [< $BitFlags Inner >]::from_bits(bits)? }) }
                unsafe fn from_bits_truncate(bits: Self::Bits) -> Self { Self { inner: [< $BitFlags Inner >]::from_bits_truncate(bits) } }
                unsafe fn from_bits_unchecked(bits: Self::Bits) -> Self { Self { inner: [< $BitFlags Inner >]::from_bits_unchecked(bits) } }
                fn is_empty(&self) -> bool { self.inner.is_empty() }
                fn is_all(&self) -> bool { self.inner.is_all() }
                fn intersects(&self, other: Self) -> bool { self.inner.intersects(other.inner) }
                fn contains(&self, other: Self) -> bool { self.inner.contains(other.inner) }
                fn insert(&mut self, other: Self) { self.inner.insert(other.inner) }
                fn remove(&mut self, other: Self) { self.inner.remove(other.inner) }
                fn toggle(&mut self, other: Self) { self.inner.toggle(other.inner) }
                fn set(&mut self, other: Self, value: bool) { self.inner.set(other.inner, value) }
            }

            impl ::core::iter::Extend<$BitFlags> for $BitFlags {
                fn extend<T: ::core::iter::IntoIterator<Item = Self>>(&mut self, iterator: T) {
                    for item in iterator {
                        self.inner.insert(item.inner)
                    }
                }
            }

            impl ::core::iter::FromIterator<$BitFlags> for $BitFlags {
                fn from_iter<T: ::core::iter::IntoIterator<Item = Self>>(iterator: T) -> $BitFlags {
                    let mut result = <Self as $crate::bitflags::BitFlags>::empty();
                    result.extend(iterator);
                    result
                }
            }
        }

        impl ::core::fmt::Debug for $BitFlags {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                self.inner.fmt(f)
            }
        }

        impl $crate::bitflags::__private::binrw::BinRead for $BitFlags {
            type Args = <$T as $crate::bitflags::__private::binrw::BinRead>::Args;

            fn read_options<R>(reader: &mut R, options: &$crate::bitflags::__private::binrw::ReadOptions, args: Self::Args) -> $crate::bitflags::__private::binrw::BinResult<Self>
            where
                R: $crate::bitflags::__private::binrw::io::Read + $crate::bitflags::__private::binrw::io::Seek,
            {
                $crate::bitflags::FlagsFrom::<$BitFlags, $T>::read_options(reader, options, args).map(|flags| *flags)
            }
        }
    };
}
