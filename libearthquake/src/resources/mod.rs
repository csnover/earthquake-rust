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
use binread::{BinRead, io::{Read, Seek}};
use bstr::BStr;
use byteordered::{Endianness, ByteOrdered};
use derive_more::{Deref, DerefMut, Index, IndexMut};
use libcommon::{encodings::DecoderRef, Reader, Resource, SeekExt, TakeSeekExt, resource::Input};
use std::{cell::RefCell, convert::{TryFrom, TryInto}, io::{Cursor, SeekFrom}};

/// A reference counted object with a vtable.
///
/// This data is always invalid on disk, but Director did the cheap thing of
/// serializing by dumping memory, so it exists nevertheless.
///
/// Since Rust has its own [`Rc`](alloc::rc::Rc) wrapper for reference counting
/// and [`dyn`] keyword for dynamic dispatch, this structure exists only for
/// serialization.
#[derive(Copy, Clone, Debug)]
pub struct Rc;

impl BinRead for Rc {
    type Args = ();

    fn read_options<R: Read + Seek>(reader: &mut R, options: &binread::ReadOptions, args: Self::Args) -> binread::BinResult<Self> {
        reader.skip(8)?;
        Ok(Self)
    }
}

#[derive(BinRead, Clone, Copy, Debug)]
#[br(import(size: u64))]
struct ByteVecHeaderV5 {
    __: Rc,
    #[br(assert(size >= used.into(), "Bad ByteVec size ({} > {})", used, size))]
    used: u32,
    capacity: u32,
    #[br(assert(header_size == Self::SIZE, "Generic ByteVec loader \
        called on specialised ByteVec with header size {}", header_size))]
    header_size: u16,
}

impl ByteVecHeaderV5 {
    const SIZE: u16 = 0x12;
}

/// A contiguous growable byte array.
#[derive(Clone, Debug, Deref, DerefMut, Index, IndexMut)]
pub struct ByteVec(Vec<u8>);

impl BinRead for ByteVec {
    type Args = ();

    fn read_options<R: Read + Seek>(input: &mut R, options: &binread::ReadOptions, args: Self::Args) -> binread::BinResult<Self> {
        let size = input.len()?;
        let header = ByteVecHeaderV5::read_options(input, options, (size, ))?;
        let data_size = u64::from(header.used) - u64::from(header.header_size);
        let mut data = Vec::with_capacity(header.capacity.try_into().unwrap());
        let bytes_read = input
            .take(data_size)
            .read_to_end(&mut data)?;
        if u64::try_from(bytes_read).unwrap() != data_size {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof).into());
        }
        Ok(Self(data))
    }
}

#[derive(BinRead, Clone, Copy, Debug)]
#[br(import(size: u64))]
#[br(assert(
    size >= u64::from(header_size) + u64::from(item_size) * u64::from(used),
    "Bad List size ({} > {})", u64::from(header_size) + u64::from(item_size) * u64::from(used), size
))]
struct ListHeaderV5 {
    __: Rc,
    used: u32,
    capacity: u32,
    header_size: u16,
    item_size: u16,
}

impl ListHeaderV5 {
    const SIZE: u16 = 0x14;
}

/// A growable list of homogenous items.
#[derive(Clone, Debug, Default, Deref, DerefMut, Index, IndexMut)]
pub struct List<T: BinRead>(Vec<T>);

impl <T: BinRead> BinRead for List<T> {
    type Args = T::Args;

    fn read_options<R: Read + Seek>(input: &mut R, options: &binread::ReadOptions, args: Self::Args) -> binread::BinResult<Self> {
        let size = input.len()?;
        let header = ListHeaderV5::read_options(input, options, (size, ))?;
        input.skip((header.header_size - ListHeaderV5::SIZE).into())?;
        let mut data = Vec::with_capacity(header.capacity.try_into().unwrap());
        let item_size = u64::from(header.item_size);
        for _ in 0..header.used {
            data.push(T::read_options(&mut input.take_seek(item_size), options, args)?);
        }
        Ok(Self(data))
    }
}

/// A growable list of heterogenous items.
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
            self.inner.load_header::<$field_type>($start_offset, $end_offset, &<_>::default())
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
            self.inner.load_entry::<$field_type>($field_index, &<_>::default())
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

/// A tuple which ties a dictionary key offset to a 32-bit value.
///
/// The offset of the key is relative to the start of the parent [`Dict`]’s
/// associated [`ByteVec`] object rather than the start of the data, so
/// knowledge of the [`ByteVec`] object’s header size is needed to get the
/// correct offset.
#[derive(BinRead, Clone, Copy, Debug)]
pub struct DictItem<T>
where
    T: TryFrom<i32>,
    T::Error: Send + Sync + 'static
{
    #[br(try_map(|value: u32| value.try_into()))]
    key_offset: usize,
    #[br(try_map(|value: i32| T::try_from(value)))]
    value: T,
}

/// An ordered dictionary with case-insensitive keys.
///
/// In Director, this keys are stored sorted case-insensitively (according
/// to the system locale) in a [`ByteVec`] for O(log n) lookups by key. Index
/// lookups are O(1), and value lookups are O(n).
///
/// The stored value is always 32-bits but can be any 32-bit value.
///
/// This is used by both `'Dict'` and `'Fmap'` resources.
#[derive(BinRead, Clone, Debug, Index, IndexMut)]
pub struct Dict<T>
where
    T: TryFrom<i32>,
    T::Error: Send + Sync + 'static,
{
    #[index] #[index_mut]
    list: List<DictItem<T>>,
    #[br(default)]
    keys: Option<ByteVec>,
}

impl <T> Dict<T>
where
    T: TryFrom<i32>,
    T::Error: Send + Sync + 'static,
{
    #[must_use]
    pub fn key_by_index(&self, index: usize) -> Option<&BStr> {
        self.keys.as_ref().and_then(|keys| {
            todo!()
        })
    }

    pub fn keys_mut(&mut self) -> &mut Option<ByteVec> {
        &mut self.keys
    }
}

// TODO: You know, finish this
// impl <Item: BinRead> Dict<Item> {
//     pub fn get_by_key(&self, key: &OsString) -> Option<usize> {
//         self.dict.get(key).copied()
//     }

//     pub fn index_of_key(&self, index: usize) -> Option<&OsString> {
//         for (k, v) in &self.dict {
//             if *v == index {
//                 return Some(k)
//             }
//         }
//         None
//     }
// }
