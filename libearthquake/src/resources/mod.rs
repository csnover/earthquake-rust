pub mod bitmap;
pub mod cast;
pub mod config;
pub mod field;
pub mod film_loop;
pub mod movie;
pub mod script;
pub mod shape;
pub mod text;
pub mod transition;
pub mod video;
pub mod xtra;

use anyhow::{Context, Result as AResult, ensure};
use byteordered::{Endianness, ByteOrdered};
use crate::ensure_sample;
use derive_more::{Deref, DerefMut, Index, IndexMut};
use libcommon::{encodings::DecoderRef, Reader, Resource, resource::Input};
use std::{cell::RefCell, convert::TryInto, io::{Cursor, Read, Seek, SeekFrom}};

#[derive(Clone, Debug, Deref, DerefMut, Index, IndexMut)]
pub struct ByteVec(Vec<u8>);

impl ByteVec {
    pub const HEADER_SIZE: u32 = 0x12;
}

impl Resource for ByteVec {
    type Context = ();
    fn load(input: &mut Input<impl Reader>, size: u32, _: &Self::Context) -> AResult<Self> {
        Rc::load(input, Rc::SIZE, &Default::default())?;
        let used = input.read_u32()?;
        let capacity = input.read_u32()?;
        let header_size = input.read_u16()?;
        let mut data = Vec::with_capacity(capacity.try_into().unwrap());
        ensure_sample!(
            used <= size,
            "Bad ByteVec size at {} ({} > {})",
            input.pos()? - u64::from(Self::HEADER_SIZE),
            used,
            size
        );
        ensure_sample!(
            header_size == Self::HEADER_SIZE.try_into().unwrap(),
            "Generic ByteVec loader called on specialised ByteVec with header size {} at {}",
            header_size,
            input.pos()? - u64::from(Self::HEADER_SIZE)
        );
        input.inner_mut().take((used - u32::from(header_size)).into()).read_to_end(&mut data)?;

        Ok(Self(data))
    }
}

#[derive(Clone, Debug, Default, Deref, DerefMut, Index, IndexMut)]
pub struct List<T: Resource>(Vec<T>);

impl <T: Resource> Resource for List<T> {
    type Context = T::Context;
    fn load(input: &mut Input<impl Reader>, size: u32, context: &Self::Context) -> AResult<Self> {
        Rc::load(input, Rc::SIZE, &Default::default())?;
        let used = input.read_u32()?;
        let capacity = input.read_u32()?;
        let header_size = u32::from(input.read_u16()?);
        let item_size = u32::from(input.read_u16()?);
        ensure_sample!(header_size + item_size * used <= size, "Bad List size at {}", input.pos()? - 0x14);
        input.skip((header_size - 0x14).into())?;
        let mut data = Vec::with_capacity(capacity.try_into().unwrap());
        for _ in 0..used {
            data.push(T::load(input, item_size, context)?);
        }

        Ok(Self(data))
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Rc;

impl Rc {
    const SIZE: u32 = 8;
}

impl Resource for Rc {
    type Context = ();
    fn load(input: &mut Input<impl Reader>, size: u32, _: &Self::Context) -> AResult<Self> {
        assert_eq!(size, Self::SIZE);
        input.skip(Self::SIZE.into())?;
        Ok(Self)
    }
}

#[derive(Debug)]
pub struct PVec {
    header_size: u32,
    offsets: Vec<u32>,
    inner: RefCell<ByteOrdered<Cursor<Vec<u8>>, Endianness>>,
    decoder: DecoderRef,
}

impl PVec {
    pub fn header_size(&self) -> u32 {
        self.header_size
    }

    pub fn is_empty(&self) -> bool {
        self.offsets.len() == 0
    }

    pub fn len(&self) -> usize {
        self.offsets.len() - 1
    }

    fn load_entry<T: Resource>(&self, index: usize, context: &T::Context) -> Option<T> {
        if index < self.offsets.len() - 1 {
            let start = self.offset(index);
            let end = self.offset(index + 1);
            self.load_offset(start.into(), end.into(), context)
        } else {
            None
        }
    }

    fn load_header<T: Resource>(&self, start: u64, end: u64, context: &T::Context) -> Option<T> {
        if end <= self.header_size().into() {
            self.load_offset(start, end, context)
        } else {
            None
        }
    }

    fn load_offset<T: Resource>(&self, start: u64, end: u64, context: &T::Context) -> Option<T> {
        if start < end {
            let mut input = self.inner.borrow_mut();
            input.seek(SeekFrom::Start(start)).unwrap();
            T::load(&mut input.as_mut(), (end - start).try_into().unwrap(), context).ok()
        } else {
            None
        }
    }

    fn offset(&self, index: usize) -> u32 {
        self.offsets[index]
    }
}

impl Resource for PVec {
    type Context = (DecoderRef, );
    fn load(input: &mut Input<impl Reader>, size: u32, context: &Self::Context) -> AResult<Self> where Self: Sized {
        const NUM_ENTRIES_SIZE: u32 = 2;
        let mut data = Vec::with_capacity(size.try_into().unwrap());
        let actual = input.take(size.into()).read_to_end(&mut data).context("Can’t read PVec into buffer")?;
        ensure!(size == actual.try_into().unwrap(), "Expected {} bytes, read {} bytes", size, actual);
        let mut inner = ByteOrdered::new(Cursor::new(data), Endianness::Big);
        let header_size = inner.read_u32().context("Can’t read PVec header size")?;
        inner.seek(SeekFrom::Start(header_size.into())).context("Can’t seek past PVec header")?;
        let num_entries = inner.read_u16().context("Can’t read number of PVec entries")?;
        let mut offsets = Vec::with_capacity(num_entries.try_into().unwrap());
        for i in 0..=num_entries {
            offsets.push(
                header_size
                + NUM_ENTRIES_SIZE
                + u32::from(num_entries + 1) * 4
                + inner.read_u32().with_context(|| format!("Can’t read PVec offset {}", i))?
            );
        }
        Ok(Self {
            inner: RefCell::new(inner),
            header_size,
            offsets,
            decoder: context.0,
        })
    }
}

#[macro_export]
macro_rules! pvec {
    (@field [offset($start_offset:literal..$end_offset:literal)], $vis:vis, $field_name:ident, $field_type:ty) => {
        $vis fn $field_name(&self) -> ::std::option::Option<$field_type> {
            #![allow(clippy::default_trait_access)]
            self.inner.load_header::<$field_type>($start_offset, $end_offset, &::std::default::Default::default())
        }
    };

    (@field [string_entry($field_index:literal, $context:expr)], $vis:vis, $field_name:ident, $field_type:ty) => {
        $vis fn $field_name(&self) -> ::std::option::Option<$field_type> {
            self.inner.load_entry::<$field_type>($field_index, &::libcommon::resource::StringContext($context, self.inner.decoder))
        }
    };

    (@field [entry($field_index:literal, $context:expr)], $vis:vis, $field_name:ident, $field_type:ty) => {
        $vis fn $field_name(&self) -> ::std::option::Option<$field_type> {
            self.inner.load_entry::<$field_type>($field_index, &$context)
        }
    };

    (@field [entry($field_index:literal)], $vis:vis, $field_name:ident, $field_type:ty) => {
        $vis fn $field_name(&self) -> ::std::option::Option<$field_type> {
            #![allow(clippy::default_trait_access)]
            self.inner.load_entry::<$field_type>($field_index, &::std::default::Default::default())
        }
    };

    (
        $(#[$outer:meta])*
        $struct_vis:vis struct $name:ident {
            $(#$attr:tt $vis:vis $n:ident: $t:ty),+$(,)?
        }
    ) => {
        $(#[$outer])*
        $struct_vis struct $name {
            inner: $crate::resources::PVec,
        }

        impl $name {
            $(
                $crate::pvec!(@field $attr, $vis, $n, $t);
            )+
        }

        impl ::std::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                let mut s = f.debug_struct(stringify!($name));
                let s = s.field("header_size", &self.inner.header_size());
                let s = s.field("num_entries", &self.inner.len());
                $(
                    let s = s.field(stringify!($n), &self.$n());
                )+
                s.finish()
            }
        }

        impl $crate::resources::Resource for $name {
            type Context = <$crate::resources::PVec as ::libcommon::Resource>::Context;
            fn load(input: &mut ::libcommon::resource::Input<impl ::libcommon::Reader>, size: u32, context: &Self::Context) -> ::anyhow::Result<Self> where Self: Sized {
                Ok(Self {
                    inner: $crate::resources::PVec::load(input, size, context)?
                })
            }
        }
    }
}
