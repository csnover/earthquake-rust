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

use binrw::{BinRead, derive_binread, io::{Read, Seek}};
use bstr::BStr;
use derive_more::{Deref, DerefMut, Index, IndexMut};
use libcommon::{SeekExt, TakeSeekExt, restore_on_error};
use std::{cmp, convert::{TryFrom, TryInto}, marker::PhantomData};

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

    fn read_options<R: Read + Seek>(reader: &mut R, _: &binrw::ReadOptions, _: Self::Args) -> binrw::BinResult<Self> {
        reader.skip(8)?;
        Ok(Self)
    }
}

#[derive(BinRead, Clone, Copy, Debug)]
#[br(import(size: u64))]
#[br(assert(
    size >= used.into(),
    "Bad ByteVec size ({} > {})", used, size
))]
#[br(assert(
    header_size == Self::SIZE,
    "Generic ByteVec loader called on specialised ByteVec with header size {}", header_size
))]
struct ByteVecHeaderV5 {
    __: Rc,
    used: u32,
    capacity: u32,
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

    fn read_options<R: Read + Seek>(input: &mut R, options: &binrw::ReadOptions, _: Self::Args) -> binrw::BinResult<Self> {
        restore_on_error(input, |input, _| {
            let size = input.bytes_left()?;
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
        })
    }
}

pub trait ListHeader: BinRead<Args = ()> + Clone + Copy + core::fmt::Debug {
    const SIZE: u16;

    fn calc_size(&self) -> u64 {
        u64::from(self.header_size()) + u64::from(self.item_size()) * u64::from(self.used())
    }

    fn header_size(&self) -> u16;

    fn item_size(&self) -> u16;

    fn used(&self) -> u32;
}

#[derive(BinRead, Clone, Copy, Debug)]
pub struct ListHeaderV5 {
    __: Rc,
    used: u32,
    capacity: u32,
    header_size: u16,
    item_size: u16,
}

impl ListHeader for ListHeaderV5 {
    const SIZE: u16 = 0x14;

    fn header_size(&self) -> u16 {
        self.header_size
    }

    fn used(&self) -> u32 {
        self.used
    }

    fn item_size(&self) -> u16 {
        self.item_size
    }
}

/// A growable list of homogeneous items with a generic header.
#[derive(Clone, Debug, Default, Deref, DerefMut, Index, IndexMut)]
pub struct List<Header: ListHeader, T: BinRead>(
    #[deref] #[deref_mut] #[index] #[index_mut]
    Vec<T>,
    PhantomData<Header>
);

/// A standard growable list of homogenous items.
pub type StdList<T> = List<ListHeaderV5, T>;

impl <Header: ListHeader, T: BinRead> BinRead for List<Header, T> {
    type Args = T::Args;

    fn read_options<R: Read + Seek>(input: &mut R, options: &binrw::ReadOptions, args: Self::Args) -> binrw::BinResult<Self> {
        let pos = input.pos()?;
        let size = input.bytes_left()?;
        let header = Header::read_options(input, options, ())?;

        if header.header_size() != Header::SIZE {
            return Err(binrw::Error::AssertFail {
                pos,
                message: format!(
                    "Incorrect List loader called on specialised List with header size {} (should be {})",
                    header.header_size(),
                    Header::SIZE,
                )
            });
        }

        let calculated_size = header.calc_size();
        if size < calculated_size {
            return Err(binrw::Error::AssertFail {
                pos,
                message: format!("Bad List size ({} > {})", calculated_size, size)
            });
        }

        let item_size = u64::from(header.item_size());
        let mut data = Vec::with_capacity(
            usize::try_from(header.used()).unwrap() * core::mem::size_of::<T>()
        );
        for _ in 0..header.used() {
            data.push(T::read_options(&mut input.take_seek(item_size), options, args.clone())?);
        }

        Ok(Self(data, PhantomData))
    }
}

/// The offset list for a growable sparse list of heterogeneous items.
///
/// Normally this is part of an object that looks like this:
///
/// ```text
/// {
///     header_size: u32,
///     < header_data >,
///     offset_table: PVecOffsets,
///     < entry_data >,
/// }
/// ```
///
/// Until Rust supports [generic associated types], it is not possible to
/// represent an object like this using [`BinRead`] without extra clones, since
/// reading the entry data requires access to all the header data and the offset
/// table, all of which is owned by the parent object. So, as a workaround, only
/// the offset table is abstracted for now, and objects of this type just use it
/// directly.
#[derive_binread]
#[derive(Clone, Debug)]
pub struct PVecOffsets(
    #[br(temp)]
    u16,
    #[br(count = self_0 + 1)]
    Vec<u32>
);

impl PVecOffsets {
    /// Returns the offset of an entry from the beginning of the data area.
    #[must_use]
    pub fn entry_offset(&self, index: usize) -> Option<u32> {
        self.0.get(index).copied()
    }

    /// Returns the size of an entry, or None if no entry exists at the given
    /// index.
    #[must_use]
    pub fn entry_size(&self, index: usize) -> Option<u32> {
        if index >= self.0.len() {
            None
        } else if index == self.0.len() - 1 {
            Some(0)
        } else {
            Some(self.0[index + 1] - self.0[index])
        }
    }

    /// Returns the size of a range of entries.
    ///
    /// If the range is out of bounds, it is automatically restricted.
    #[must_use]
    pub fn entry_range_size<Range: std::ops::RangeBounds<usize>>(&self, range: Range) -> u32 {
        let max = cmp::min(self.len(), match range.end_bound() {
            std::ops::Bound::Included(value) => value + 1,
            std::ops::Bound::Excluded(value) => *value,
            std::ops::Bound::Unbounded => usize::MAX,
        });
        let min = cmp::max(0, match range.start_bound() {
            std::ops::Bound::Included(value) => *value,
            std::ops::Bound::Excluded(value) => value + 1,
            std::ops::Bound::Unbounded => 0,
        });
        self.0[max] - self.0[min]
    }

    /// Returns whether or not an entry exists.
    #[must_use]
    pub fn has_entry(&self, index: usize) -> bool {
        self.entry_size(index).unwrap_or(0) != 0
    }

    /// Returns `true` if there are no entries in `self`.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.len() == 1
    }

    /// Returns the number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len() - 1
    }

    /// Creates a new `PVecOffsets` by copying from the given position and
    /// count.
    ///
    /// This is a hack to work around the lack of generic associated types.
    pub fn slice(&self, at: usize, count: usize) -> Self {
        // `+ 1` for the terminator entry
        Self(self.0[at..at + count + 1].to_vec())
    }
}

#[macro_export]
macro_rules! pvec {
    // Final entry rule.
    (@entry $offsets:ident {
        $(,)?
    } -> { $($output:tt)* }
    ) => {
        $crate::pvec! { @write -> { $($output)* } }
    };

    // Skips any remaining entries that were not included in the struct.
    // `entry_num..`
    (@entry $offsets:ident {
        $entry_num:literal.. $($tail:tt)*
    } -> { $($output:tt)* }
    ) => {
        $crate::pvec! { @write -> {
            $($output)*
            #[br(pad_before = i64::from($offsets.entry_range_size($entry_num..)))]
            #[br(ignore)]
            _end: ();
        } }
    };

    // Skips a single entry without assigning it to anything.
    // `entry_num => _`
    (@entry $offsets:ident {
        $entry_num:literal => _ $($tail:tt)*
    } -> { $($output:tt)* }
    ) => {
        $crate::pvec! { @entry $offsets {
            $entry_num..=$entry_num => _
            $($tail)*
        } -> { $($output)* } }
    };

    // Skips a range of entries without assigning them to anything.
    // `entry_range => _`
    (@entry $offsets:ident {
        $entry_range:pat => _ $(, $($tail:tt)*)?
    } -> { $($output:tt)* }
    ) => {
        $crate::pvec! { @entry $offsets { $($($tail)*)? } -> {
            $($output)*
            #[br(pad_before = i64::from($offsets.entry_range_size($entry_range)))]
        } }
    };

    // Delegates reading of entries to an inner type.
    // `#[attr] _ => entry_name: entry_type`
    (@entry $offsets:ident {
        $(#[$entry_meta:meta])*
        _ => $entry_ident:ident : $entry_ty:ty
        $(,)?
    } -> { $($output:tt)* }
    ) => {
        $crate::pvec! { @write -> {
            $($output)*
            $(#[$entry_meta])*
            $entry_ident: $entry_ty;
        } }
    };

    // Assigns an entry with the given entry number to a struct field.
    // `#[attr] entry_num => entry_name: entry_type`
    (@entry $offsets:ident {
        $(#[$entry_meta:meta])*
        $entry_num:literal => $entry_ident:ident : $entry_ty:ty
        $(, $($tail:tt)*)?
    } -> { $($output:tt)* }
    ) => {
        $crate::pvec! { @entry $offsets { $($($tail)*)? } -> {
            $($output)*
            $(#[$entry_meta])*
            #[br(
                if($offsets.has_entry($entry_num)),
                pad_size_to($offsets.entry_size($entry_num).unwrap_or(0))
            )]
            $entry_ident: Option<$entry_ty>;
        } }
    };

    // Writes the final struct.
    (@write -> {
        $(#[$meta:meta])* $vis:vis struct $ident:ident;
        $($(#[$field_meta:meta])* $field_ident:ident : $field_ty:ty;)*
    }) => {
        #[derive(binrw::BinRead)]
        #[br(big)]
        $(#[$meta])*
        $vis struct $ident {
            $($(#[$field_meta])* $field_ident: $field_ty),*
        }
    };

    // Entrypoint. Reads the non-entries portions of the struct.
    (
        $(#[$meta:meta])*
        $vis:vis struct $ident:ident {
            header {
                $($(#[$field_meta:meta])* $field_ident:ident : $field_ty:ty),*
                $(,)?
            }

            // Required, due to macro hygiene, for the caller to be able to
            // access the `offsets` field, which is generated within the macro.
            offsets = $offsets:ident;

            entries {
                $($tail:tt)*
            }
        }
    ) => {
        $crate::pvec! { @entry $offsets { $($tail)* } -> {
            $(#[$meta])* $vis struct $ident;
            header_size: u32;
            $($(#[$field_meta])* $field_ident: $field_ty;)*
            $offsets: $crate::resources::PVecOffsets;
        } }
    };
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
    T: TryFrom<i32> + 'static,
    T::Error: std::error::Error + Send + Sync + 'static
{
    #[br(try_map(|value: u32| value.try_into()))]
    key_offset: usize,
    #[br(try_map(|value: i32| T::try_from(value)))]
    value: T,
}

#[derive(BinRead, Clone, Copy, Debug)]
pub struct DictListHeaderV5(#[br(pad_after = 8)] ListHeaderV5);

impl ListHeader for DictListHeaderV5 {
    const SIZE: u16 = 0x1c;

    fn header_size(&self) -> u16 {
        self.0.header_size
    }

    fn item_size(&self) -> u16 {
        self.0.item_size
    }

    fn used(&self) -> u32 {
        self.0.used
    }
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
    T: TryFrom<i32> + 'static,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    #[index] #[index_mut]
    list: List<DictListHeaderV5, DictItem<T>>,
    #[br(default)]
    keys: Option<ByteVec>,
}

impl <T> Dict<T>
where
    T: TryFrom<i32>,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    #[must_use]
    pub fn key_by_index(&self, index: usize) -> Option<&BStr> {
        let keys = self.keys.as_ref()?;
        let offset = self.list.get(index)?.key_offset - usize::from(ByteVecHeaderV5::SIZE);
        let size = usize::try_from(
            u32::from_be_bytes(keys.get(offset..offset + 4)?.try_into().unwrap())
        ).unwrap();
        keys.get(offset + 4..offset + 4 + size).map(|b| b.into())
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
