use anyhow::{anyhow, bail, Context, ensure, Result as AResult};
use binread::BinRead;
use bitflags::bitflags;
use byteorder::{ByteOrder, BigEndian};
use byteordered::{ByteOrdered, Endianness};
use crate::{ApplicationVise, OSType, ResourceId};
use derive_more::Display;
use libcommon::{Reader, encodings::MAC_ROMAN, string::ReadExt, binread_flags};
use std::{any::Any, cell::RefCell, convert::{TryFrom, TryInto}, io::{Cursor, Read, Seek, SeekFrom}, rc::{Weak, Rc}, sync::atomic::{Ordering, AtomicI16}};

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq)]
pub struct RefNum(pub i16);
static REF_NUM: AtomicI16 = AtomicI16::new(1);

pub trait ResourceSource {
    fn contains(&self, id: impl Into<ResourceId>) -> bool;
    fn load<R: 'static + libcommon::Resource>(&self, id: ResourceId, context: &R::Context) -> AResult<Rc<R>>;
}

#[derive(Debug)]
/// A Macintosh Resource File Format file reader.
pub struct ResourceFile<T: Reader> {
    input: RefCell<ByteOrdered<T, Endianness>>,
    decompressor: RefCell<DecompressorState>,
    resource_map: ResourceMap,
}

impl<T: Reader> ResourceFile<T> {
    /// Makes a new `ResourceFile` from a readable stream.
    pub fn new(data: T) -> AResult<Self> {
        let mut input = ByteOrdered::new(data, Endianness::Big);

        let _data_offset = input.read_u32().context("Can’t read data offset")?;
        let map_offset = input.read_u32().context("Can’t read map offset")?;
        let _data_size = input.read_u32().context("Can’t read data size")?;
        let map_size = input.read_u32().context("Can’t read map size")?;
        ensure!(map_size >= 30, "Bad resource map size {}", map_size);

        input.seek(SeekFrom::Start(map_offset.into()))
            .map_err(|_| anyhow!("Bad resource map offset {}", map_offset))?;

        let resource_map = ResourceMap::read(&mut input)
            .context("Bad resource map")?;

        Ok(Self {
            input: RefCell::new(input),
            decompressor: RefCell::new(DecompressorState::Waiting),
            resource_map,
        })
    }

    /// Returns the number of resources with the given `OSType`.
    pub fn count(&self, os_type: impl Into<OSType>) -> i16 {
        self.find_kind(os_type).map_or(0, |kind| kind.count)
    }

    pub fn id_of_name(&self, os_type: impl Into<OSType>, name: impl AsRef<[u8]>) -> Option<ResourceId> {
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

    pub fn id_of_index(&self, os_type: impl Into<OSType>, index: i16) -> Option<ResourceId> {
        let os_type = os_type.into();
        self.find_kind(os_type)
            .and_then(|kind| kind.resources.get(usize::try_from(index).unwrap()))
            .map(|res| ResourceId::new(os_type, res.id))
    }

    pub fn into_inner(self) -> T {
        self.input.into_inner().into_inner()
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = ResourceId> + 'a {
        self.resource_map.kinds.iter().flat_map(|k| {
            let os_type = k.kind;
            k.resources.iter().map(move |r| ResourceId::new(os_type, r.id))
        })
    }

    pub fn iter_kind<'a>(&'a self, os_type: impl Into<OSType>) -> impl Iterator<Item = ResourceId> + 'a {
        let os_type = os_type.into();
        self.find_kind(os_type)
            .into_iter()
            .flat_map(move |kind| kind.resources.iter().map(move |r| ResourceId::new(os_type, r.id)))
    }

    /// Returns the name embedded in the Resource File. For applications, this
    /// is the name of the application.
    pub fn name(&self) -> Option<String> {
        let mut input = self.input.borrow_mut();
        input.seek(SeekFrom::Start(0x30)).ok()?;
        // TODO: Do not assume MAC_ROMAN
        input.read_pascal_str(MAC_ROMAN).ok()
    }

    pub fn reference_number(&self) -> RefNum {
        self.resource_map.ref_num
    }

    fn decompress(&self, data: &[u8]) -> AResult<Vec<u8>> {
        // https://stackoverflow.com/questions/33495933/how-to-end-a-borrow-in-a-match-or-if-let-expression
        let resource_id = if let DecompressorState::Waiting = *self.decompressor.borrow() {
            self.find_kind(b"CODE")
                .and_then(|kind| kind.resources.last())
                .map(|resource| ResourceId::new(b"CODE", resource.id))
        } else {
            None
        };

        if let Some(resource_id) = resource_id {
            let resource_data = self.load::<Vec<u8>>(resource_id, &())
                .context("Can’t find the Application VISE CODE resource")?;
            let shared_data = ApplicationVise::find_shared_data(&resource_data)
                .context("Can’t find the Application VISE shared dictionary")?;
            self.decompressor.replace(DecompressorState::Loaded(ApplicationVise::new(shared_data.to_vec())));
        }

        if let DecompressorState::Loaded(decompressor) = &*self.decompressor.borrow() {
            decompressor.decompress(&data).context("Decompression failure")
        } else {
            bail!("Missing decompressor")
        }
    }

    fn find_item(&self, id: ResourceId) -> Option<&ResourceItem> {
        self.find_kind(id.os_type())
            .and_then(|kind| {
                kind.resources.iter().find(|&res| res.id == id.id())
            })
    }

    fn find_kind(&self, os_type: impl Into<OSType>) -> Option<&ResourceKind> {
        let os_type = os_type.into();
        self.resource_map.kinds.iter().find(move |&kind| kind.kind == os_type)
    }
}

impl <T: Reader> ResourceSource for ResourceFile<T> {
    fn contains(&self, id: impl Into<ResourceId>) -> bool {
        self.find_item(id.into()).is_some()
    }

    fn load<R: 'static + libcommon::Resource>(&self, id: ResourceId, context: &R::Context) -> AResult<Rc<R>> {
        let entry = self.find_item(id)
            .with_context(|| format!("Resource {} not found", id))?;

        ensure!(!entry.flags.contains(ResourceFlags::COMPRESSED), "Resource {} uses unsupported compression", id);

        if let Some(data) = entry.data.borrow().as_ref().and_then(Weak::upgrade) {
            return data.downcast::<R>()
                .map_err(|_| anyhow!("Invalid data type for resource {}", id));
        }

        let mut input = self.input.try_borrow_mut()?;
        input.seek(SeekFrom::Start(entry.data_offset.into()))
            .with_context(|| format!("Can’t seek to resource {}", id))?;

        let size = input.read_u32()
            .with_context(|| format!("Can’t read size of resource {}", id))?;

        let is_vise_compressed = {
            let mut sig = [ 0; 4 ];
            input.read_exact(&mut sig).ok();
            input.seek(SeekFrom::Start((entry.data_offset + 4).into()))
                .with_context(|| format!("Can’t seek to resource {}", id))?;
            ApplicationVise::is_compressed(&sig)
        };

        if is_vise_compressed {
            let data = {
                let mut compressed_data = Vec::with_capacity(size.try_into().unwrap());
                input.as_mut().take(size.into()).read_to_end(&mut compressed_data)?;
                self.decompress(&compressed_data)
                    .with_context(|| format!("Can’t decompress resource {}", id))?
            };
            let decompressed_size = u32::try_from(data.len()).unwrap();
            R::load(&mut ByteOrdered::new(Cursor::new(data), Endianness::Big), decompressed_size, context)
        } else {
            R::load(&mut input.as_mut(), size, context)
        }.map(|resource| {
            let resource = Rc::new(resource);
            *entry.data.borrow_mut() = Some(Rc::downgrade(&(Rc::clone(&resource) as Rc<dyn Any>)));
            resource
        })
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

binread_flags!(ResourceFlags, u8);

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
    type_list_offset: u16,
    name_list_offset: u16,
    #[br(assert(count < 2727, anyhow!("Bad resource kind count")), map = |count: i16| count + 1)]
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
    kind: OSType,
    #[br(assert(count < 2727, anyhow!("Bad resource count")), map = |count: i16| count + 1)]
    count: i16,
    #[br(args(data_offset), count(count), offset((map_offset + 28).into()), parse_with = binread::FilePtr16::parse)]
    resources: Vec<ResourceItem>,
}

fn parse_u24<R: binread::io::Read + binread::io::Seek>(reader: &mut R, _: &binread::ReadOptions, _: ()) -> binread::BinResult<u32> {
    let mut bytes = [ 0; 3 ];
    reader.read_exact(&mut bytes)?;
    Ok(BigEndian::read_u24(&bytes))
}

#[derive(BinRead, Debug)]
#[br(big, import(data_offset: u32))]
struct ResourceItem {
    id: i16,
    name_offset: i16,
    flags: ResourceFlags,
    #[br(map = |offset: u32| offset + data_offset, parse_with = parse_u24)]
    data_offset: u32,
    #[br(pad_before(4), default)]
    data: RefCell<Option<Weak<dyn Any>>>,
}

#[cfg(test)]
mod tests {
    // TODO
}
