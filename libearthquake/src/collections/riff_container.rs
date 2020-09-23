use anyhow::{Context, Result as AResult};
use byteorder::{ByteOrder, BigEndian};
use byteordered::Endianness;
use crate::{
    ensure_sample,
    resources::{ByteVec, List},
};
use libcommon::{
    Reader,
    Resource,
    resource::Input,
encodings::DecoderRef, encodings::MAC_ROMAN, encodings::Decoder};
use libmactoolbox::os;
use derive_more::{Deref, Index};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::{collections::HashMap, ffi::OsString, rc::Rc};
use super::riff::{ChunkIndex, Riff};

#[derive(Copy, Clone, Debug, Eq, PartialEq, FromPrimitive)]
pub enum ChunkFileKind {
    Movie,
    Cast,
    Xtra,
}

#[derive(Clone, Copy, Debug)]
pub struct ChunkFile {
    chunk_index: ChunkIndex,
    kind: ChunkFileKind,
}
impl ChunkFile {
    pub fn chunk_index(&mut self) -> ChunkIndex {
        self.chunk_index
    }

    pub fn kind(&mut self) -> ChunkFileKind {
        self.kind
    }
}
impl Resource for ChunkFile {
    type Context = ();
    fn load(input: &mut Input<impl Reader>, size: u32, _: &Self::Context) -> AResult<Self> {
        let pos = input.pos()?;
        ensure_sample!(size >= 4 && size <= 8, "Bad ChunkFile size at {} ({})", pos, size);
        let chunk_index = ChunkIndex::new(input.read_i32()?);
        let kind = if size == 4 {
            ChunkFileKind::Movie
        } else {
            let bits = input.read_u32()?;
            ChunkFileKind::from_u32(bits)
                .with_context(|| format!("Bad ChunkFile kind at {} ({})", pos, bits))?
        };
        Ok(Self { chunk_index, kind })
    }
}

#[derive(Clone, Copy, Debug)]
struct DictItem {
    /// The offset of the key is relative to the start of the ByteVec object
    /// rather than the start of the data, so knowledge of ByteVec object header
    /// size is needed to get the correct offset.
    key_offset: u32,
    value: i32,
}
impl Resource for DictItem {
    type Context = ();
    fn load(input: &mut Input<impl Reader>, size: u32, _: &Self::Context) -> AResult<Self> {
        ensure_sample!(size == 8, "Bad DictItem size at {} ({} != 8)", size, input.pos()?);
        let key_offset = input.read_u32()?;
        let value = input.read_i32()?;
        Ok(Self { key_offset, value })
    }
}

#[derive(Clone, Debug, Index)]
struct Dict {
    #[index]
    list: List<DictItem>,
    // TODO: Lookups should be case-insensitive
    dict: HashMap<OsString, usize>,
}
impl Dict {
    pub fn get_by_key(&self, key: &OsString) -> Option<usize> {
        self.dict.get(key).copied()
    }

    pub fn index_of_key(&self, index: usize) -> Option<&OsString> {
        for (k, v) in &self.dict {
            if *v == index {
                return Some(k)
            }
        }
        None
    }
}
impl Resource for Dict {
    type Context = DecoderRef;
    fn load(input: &mut Input<impl Reader>, size: u32, context: &Self::Context) -> AResult<Self> {
        let mut input = input.as_mut().into_endianness(Endianness::Big);
        let list_size = input.read_u32()?;
        let keys_size = input.read_u32()?;
        ensure_sample!(list_size + keys_size <= size, "Bad Dict size at {} ({} > {})", input.pos()? - 8, list_size + keys_size, size);

        let list = List::<DictItem>::load(&mut input, list_size, &<List::<DictItem> as Resource>::Context::default())?;
        let keys = ByteVec::load(&mut input, keys_size, &<ByteVec as Resource>::Context::default())?;
        let mut dict = HashMap::new();

        for item in list.iter() {
            let start = item.key_offset as usize - ByteVec::HEADER_SIZE;
            let end = start + 4;
            let size = BigEndian::read_u32(&keys[start..end]) as usize;
            let key = OsString::from(context.decode(&keys[end..end + size]));
            dict.insert(key, item.value as usize);
        }

        Ok(Self { list, dict })
    }
}

#[derive(Clone, Debug, Deref, Index)]
pub struct RiffContainer<T: Reader> {
    riff: Rc<Riff<T>>,
    #[deref]
    #[index]
    file_list: List<ChunkFile>,
    file_dict: Dict,
}

impl <T: Reader> RiffContainer<T> {
    pub fn new(input: T) -> AResult<Self> {
        let riff = Riff::new(input).context("Bad RIFF container")?;
        let file_list = riff.load::<List<ChunkFile>>(riff.first_of_kind(os!(b"List")), &Default::default()).context("Bad List chunk")?;
        let file_dict = riff.load::<Dict>(riff.first_of_kind(os!(b"Dict")), &(MAC_ROMAN as &dyn Decoder)).context("Bad Dict chunk")?;

        Ok(Self {
            riff: Rc::new(riff),
            file_list: Rc::try_unwrap(file_list).unwrap(),
            file_dict: Rc::try_unwrap(file_dict).unwrap(),
        })
    }

    #[must_use]
    pub fn filename(&self, index: usize) -> Option<&OsString> {
        self.file_dict.index_of_key(index)
    }

    #[must_use]
    pub fn kind(&self, index: usize) -> Option<ChunkFileKind> {
        self.file_list.get(index).map(|i| i.kind)
    }

    pub fn load_file(&self, index: usize) -> AResult<Riff<T>> {
        self.riff.load_riff(self.file_list[index].chunk_index)
    }
}
