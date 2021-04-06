//! Type definitions for [Mac Resource Files].
//!
//! [Mac Resource Files]: https://developer.apple.com/library/archive/documentation/mac/pdf/MoreMacintoshToolbox.pdf#page=151

use binrw::{BinRead, io::{Cursor, Read, SeekFrom, self}};
use byteorder::{ByteOrder, BigEndian};
use crate::types::{MacString, PString};
use derive_more::Display;
use libcommon::{SeekExt, TakeSeekExt, bitflags::BitFlags, bitflags};
use std::{any::Any, cell::RefCell, convert::{TryFrom, TryInto}, rc::{Weak, Rc}, sync::atomic::{Ordering, AtomicI16}};
use super::{ApplicationVise, Error as ResourceError, OsType, Result, ResNum, ResourceId, Source};

/// A file reference number which corresponds to an open resource fork.
#[derive(Clone, Copy, Debug, Default, Display, Eq, PartialEq)]
pub struct RefNum(pub i16);

// TODO: This should eventually be associated with the resource manager so it
// can be serialised along with everything else.
static REF_NUM: AtomicI16 = AtomicI16::new(1);

#[derive(Debug)]
/// A Macintosh Resource File Format file reader.
pub struct File<T: io::Read + io::Seek> {
    input: RefCell<T>,
    decompressor: RefCell<DecompressorState>,
    resource_map: ResourceMap,
}

#[derive(BinRead, Debug)]
#[br(big)]
struct Header {
    data_offset: u32,
    map_offset: u32,
    data_size: u32,
    #[br(assert(map_size >= 30, ResourceError::BadMapSize(map_size)))]
    map_size: u32,
}

impl<T: io::Read + io::Seek> File<T> {
    /// Makes a new `ResourceFile` from a stream.
    ///
    /// The input stream should be restricted to the range of the resource file
    /// data.
    pub fn new(mut data: T) -> Result<Self> {
        let file_size = data.bytes_left()?;
        let header = Header::read(data.by_ref())?;
        let min_file_size = u64::from(core::cmp::max(
            header.map_offset.saturating_add(header.map_size),
            header.data_offset.saturating_add(header.data_size)
        ));

        if file_size < min_file_size {
            return Err(ResourceError::FileTooSmall(file_size, min_file_size));
        }

        data.seek(SeekFrom::Start(header.map_offset.into()))?;

        let resource_map = ResourceMap::read(data.by_ref())?;

        Ok(Self {
            input: RefCell::new(data),
            decompressor: RefCell::new(DecompressorState::Waiting),
            resource_map,
        })
    }

    /// Returns the number of resources with the given [`OSType`].
    pub fn count(&self, os_type: impl Into<OsType>) -> i16 {
        self.find_kind(os_type).map_or(0, |kind| kind.count)
    }

    /// Returns a resource ID for the named resource with the given [`OSType`].
    ///
    /// # Panics
    ///
    /// Panics if a name offset in the resource data is negative, other than the
    /// “no name” sentinel.
    pub fn id_of_name(&self, os_type: impl Into<OsType>, name: impl AsRef<[u8]>) -> Option<ResourceId> {
        let os_type = os_type.into();
        self.find_kind(os_type)
            .and_then(|kind| kind.resources.iter().find(|res| {
                if res.name_offset == -1 {
                    return false;
                }

                let start = usize::try_from(res.name_offset).unwrap();
                let end = start + usize::from(self.resource_map.names[start]);
                *name.as_ref() == self.resource_map.names[start + 1..=end]
            }))
            .map(|res| ResourceId::new(os_type, res.id))
    }

    /// Returns the [`ResourceId`] of a resource with the given type and index.
    ///
    /// # Panics
    ///
    /// Panics if the given index is negative.
    pub fn id_of_index(&self, os_type: impl Into<OsType>, index: i16) -> Option<ResourceId> {
        let os_type = os_type.into();
        self.find_kind(os_type)
            .and_then(|kind| kind.resources.get(usize::try_from(index).unwrap()))
            .map(|res| ResourceId::new(os_type, res.id))
    }

    /// Consumes the `ResourceFile`, returning the wrapped reader.
    pub fn into_inner(self) -> T {
        self.input.into_inner()
    }

    /// Returns an iterator over all resource IDs.
    pub fn iter(&self) -> impl Iterator<Item = ResourceId> + '_ {
        self.resource_map.kinds.iter().flat_map(|k| {
            let os_type = k.kind;
            k.resources.iter().map(move |r| ResourceId::new(os_type, r.id))
        })
    }

    /// Returns an iterator over all resource IDs with the given type.
    pub fn iter_kind(&self, os_type: impl Into<OsType>) -> impl Iterator<Item = ResourceId> + '_ {
        let os_type = os_type.into();
        self.find_kind(os_type)
            .into_iter()
            .flat_map(move |kind| kind.resources.iter().map(move |r| ResourceId::new(os_type, r.id)))
    }

    /// Returns the name embedded in the Resource File. For applications, this
    /// is the name of the application.
    pub fn name(&self) -> Option<MacString> {
        let mut input = self.input.try_borrow_mut().ok()?;
        input.seek(SeekFrom::Start(0x30)).ok()?;
        PString::read(input.by_ref()).ok().map(<_>::into)
    }

    pub fn reference_number(&self) -> RefNum {
        self.resource_map.ref_num
    }

    fn decompress(&self, for_id: ResourceId, data: &[u8]) -> Result<Vec<u8>> {
        // TODO: is this still needed?
        // https://stackoverflow.com/questions/33495933/how-to-end-a-borrow-in-a-match-or-if-let-expression
        let resource_id = if let DecompressorState::Waiting = *self.decompressor.borrow() {
            self.find_kind(b"CODE")
                .and_then(|kind| kind.resources.last())
                .map(|resource| ResourceId::new(b"CODE", resource.id))
        } else {
            None
        };

        if let Some(resource_id) = resource_id {
            let resource_data = self.load::<Vec<u8>>(resource_id)
                .map_err(|_| ResourceError::MissingViseResource)?;
            let shared_data = ApplicationVise::find_shared_data(&resource_data)
                .ok_or(ResourceError::MissingViseDictionary)?;
            self.decompressor.replace(DecompressorState::Loaded(ApplicationVise::new(shared_data.to_vec())));
        }

        if let DecompressorState::Loaded(decompressor) = &*self.decompressor.borrow() {
            decompressor.decompress(&data).map_err(|error| ResourceError::BadCompression(for_id, error))
        } else {
            Err(ResourceError::MissingDecompressor)
        }
    }

    fn find_item(&self, id: ResourceId) -> Option<&ResourceItem> {
        self.find_kind(id.os_type())
            .and_then(|kind| {
                kind.resources.iter().find(|&res| res.id == id.id())
            })
    }

    fn find_kind(&self, os_type: impl Into<OsType>) -> Option<&ResourceKind> {
        let os_type = os_type.into();
        self.resource_map.kinds.iter().find(move |&kind| kind.kind == os_type)
    }
}

impl <T: io::Read + io::Seek> Source for File<T> {
    fn contains(&self, id: impl Into<ResourceId>) -> bool {
        self.find_item(id.into()).is_some()
    }

    fn load_args<R: BinRead + 'static>(&self, id: ResourceId, args: R::Args) -> Result<Rc<R>> {
        let mut options = binrw::ReadOptions::default();
        options.endian = binrw::Endian::Big;

        let entry = self.find_item(id).ok_or(ResourceError::NotFound(id))?;

        if entry.flags.contains(ResourceFlags::COMPRESSED) {
            return Err(ResourceError::UnsupportedCompression(id));
        }

        if let Some(data) = entry.data.borrow().as_ref().and_then(Weak::upgrade) {
            return data.downcast::<R>()
                .map_err(|_| ResourceError::BadDataType(id));
        }

        let mut input = self.input.try_borrow_mut()?;
        input.seek(SeekFrom::Start(entry.data_offset.into()))
            .map_err(|error| ResourceError::SeekFailure(id, error))?;

        let size = u32::read_options(input.by_ref(), &options, ())
            .map_err(|error| ResourceError::ReadSizeFailure(id, match error {
                binrw::Error::Io(error) => error,
                _ => unreachable!("primitive reads cannot fail except by i/o error")
            }))?;

        let is_vise_compressed = {
            let mut sig = [ 0; 4 ];

            // A read error here is OK because that just means the resource is
            // probably small, and definitely not compressed
            input.read_exact(&mut sig).ok();

            // Since we only read to check for a VISE signature, seek back to
            // the start of data; absolute seek because the read may or may not
            // have succeeded
            input.seek(SeekFrom::Start((entry.data_offset + 4).into()))
                .map_err(|error| ResourceError::SeekFailure(id, error))?;

            ApplicationVise::is_compressed(&sig)
        };

        let resource = Rc::new(if is_vise_compressed {
            let data = {
                let mut compressed_data = Vec::with_capacity(size.try_into().unwrap());
                input.by_ref().take(size.into()).read_to_end(&mut compressed_data)?;
                self.decompress(id, &compressed_data)?
            };
            R::read_options(&mut Cursor::new(data), &options, args)
        } else {
            R::read_options(&mut input.by_ref().take_seek(size.into()), &options, args)
        }.map_err(|error| ResourceError::ResourceReadFailure(id, error))?);

        *entry.data.borrow_mut() = Some(Rc::downgrade(&(Rc::clone(&resource) as _)));
        Ok(resource)
    }
}

bitflags! {
    /// The flags set on a resource from a Resource File.
    pub struct ResourceFlags: u8 {
        /// Reserved; unused.
        const RESERVED            = 0x80;

        /// The resource should be loaded in the system heap instead of the
        /// application heap.
        const LOAD_TO_SYSTEM_HEAP = 0x40;

        /// The resource may be paged out of memory.
        const PURGEABLE           = 0x20;

        /// The resource may not be moved in memory.
        const LOCKED              = 0x10;

        /// The resource is read-only.
        const READ_ONLY           = 0x08;

        /// The resource should be loaded as soon as the file is opened.
        const PRELOAD             = 0x04;

        /// An internal flag used by the Resource Manager.
        const CHANGED             = 0x02;

        /// The resource data is compressed.
        const COMPRESSED          = 0x01;
    }
}

#[derive(Debug)]
enum DecompressorState {
    Waiting,
    Loaded(ApplicationVise),
}

#[derive(BinRead, Debug)]
#[br(big)]
struct ResourceMap {
    data_offset: u32,
    map_offset: u32,
    data_size: u32,
    map_size: u32,
    _next_map_handle: u32,
    #[br(pad_before(2), calc = RefNum(REF_NUM.fetch_add(1, Ordering::Relaxed)))]
    ref_num: RefNum,
    _attributes: i16,
    #[br(assert(type_list_offset >= 28))]
    type_list_offset: u16,
    name_list_offset: u16,
    #[br(map = |count: i16| count + 1)]
    #[br(assert(count < 2727, ResourceError::BadMapKindCount(count)))]
    count: i16,
    #[br(args(data_offset, map_offset), count(count), offset(u64::from(type_list_offset) - 28))]
    kinds: Vec<ResourceKind>,
    #[br(count(map_size - u32::from(name_list_offset)), seek_before(SeekFrom::Start((map_offset + u32::from(name_list_offset)).into())))]
    names: Vec<u8>,
}

#[derive(BinRead, Debug)]
#[br(big, import(data_offset: u32, map_offset: u32))]
struct ResourceKind {
    #[br(map = |b: u32| b.into())]
    kind: OsType,
    #[br(map = |count: i16| count + 1)]
    #[br(assert(count < 2727, ResourceError::BadMapResourceCount(count, kind)))]
    count: i16,
    #[br(args(data_offset), count(count), offset((map_offset + 28).into()), parse_with = binrw::FilePtr16::parse)]
    resources: Vec<ResourceItem>,
}

// TODO: Uplift to binrw
fn parse_u24<R: io::Read + io::Seek>(reader: &mut R, _: &binrw::ReadOptions, _: ()) -> binrw::BinResult<u32> {
    let mut bytes = [ 0; 3 ];
    reader.read_exact(&mut bytes)?;
    Ok(BigEndian::read_u24(&bytes))
}

#[derive(BinRead, Debug)]
#[br(big, import(data_offset: u32))]
struct ResourceItem {
    id: ResNum,
    name_offset: i16,
    flags: ResourceFlags,
    #[br(map = |offset: u32| offset + data_offset, parse_with = parse_u24)]
    data_offset: u32,
    #[br(pad_before(4), default)]
    data: RefCell<Option<Weak<dyn Any>>>,
}
