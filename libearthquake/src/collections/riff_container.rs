use anyhow::{Context, Result as AResult};
use binrw::BinRead;
use bstr::BStr;
use crate::resources::{ByteVec, StdList};
use libcommon::{Reader, SeekExt, restore_on_error};
use derive_more::{Deref, DerefMut, Index, IndexMut};
use smart_default::SmartDefault;
use std::{io::{Read, Seek}, rc::Rc};
use super::riff::{ChunkIndex, Riff, RiffResult};

/// An index entry for a file embedded within a [`RiffContainer`].
#[derive(BinRead, Copy, Clone, Debug, Eq, PartialEq, SmartDefault)]
#[br(repr(u32))]
pub enum ChunkFileKind {
    #[default]
    Movie,
    Cast,
    Xtra,
}

#[derive(Clone, Copy, Debug)]
pub struct ChunkFile {
    /// The index of the chunk containing the file.
    chunk_index: ChunkIndex,
    /// The kind of the file.
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

impl BinRead for ChunkFile {
    type Args = ();

    fn read_options<R: Read + Seek>(input: &mut R, options: &binrw::ReadOptions, _: Self::Args) -> binrw::BinResult<Self> {
        restore_on_error(input, |input, pos| {
            let mut options = *options;
            options.endian = binrw::Endian::Big;

            let size = input.bytes_left()?;
            if !(4..=8).contains(&size) {
                return Err(binrw::Error::AssertFail {
                    pos,
                    message: format!("Bad ChunkFile size {}", size)
                });
            }
            let chunk_index = ChunkIndex::read_options(input, &options, ())?;
            let kind = if size == 4 {
                ChunkFileKind::Movie
            } else {
                ChunkFileKind::read_options(input, &options, ())?
            };

            Ok(Self { chunk_index, kind })
        })
    }
}

/// A node which ties raw dictionary data to its associated list entry in the
/// dictionary.
#[derive(BinRead, Clone, Copy, Debug)]
struct DictItem {
    /// The offset of the key is relative to the start of the ByteVec object
    /// rather than the start of the data, so knowledge of ByteVec object header
    /// size is needed to get the correct offset.
    key_offset: u32,
    list_index: i32,
}

type InnerDict = crate::resources::Dict<usize>;

/// A serialized [`Dict`].
#[derive(Clone, Debug, Deref, DerefMut)]
struct Dict(InnerDict);

impl BinRead for Dict {
    type Args = ();

    fn read_options<R: binrw::io::Read + binrw::io::Seek>(input: &mut R, options: &binrw::ReadOptions, args: Self::Args) -> binrw::BinResult<Self> {
        restore_on_error(input, |input, pos| {
            let mut options = *options;
            options.endian = binrw::Endian::Big;

            let size = input.bytes_left()?;
            let (dict_size, keys_size) = <(u32, u32)>::read_options(input, &options, args)?;
            let expected_size = u64::from(dict_size + keys_size);
            if expected_size > size {
                return Err(binrw::Error::AssertFail {
                    pos,
                    message: format!("Bad Dict size at {} ({} > {})", pos, expected_size, size)
                });
            }

            let (mut dict, keys) = <(InnerDict, ByteVec)>::read_options(input, &options, args)?;
            dict.keys_mut().replace(keys);

            Ok(Self(dict))
        })
    }
}

/// A RIFF file used as a container for other files.
///
/// In Director 4+, one RIFF file is used per movie or cast library. When
/// packaged for release in a projector, movies and cast added to the projector
/// are embedded in a RIFF container, identified with the `APPL` subtype. This
/// container embeds each file as a separate chunk and includes several index
/// chunks which describe the original files’ names and the order in which they
/// were added to the container so they can be played in sequence.
///
/// Starting in Director 6 (TODO: maybe 7? check this), the container was
/// extended to also include binary Xtras.
#[derive(Clone, Debug, Deref, DerefMut, Index, IndexMut)]
pub struct RiffContainer<T: Reader> {
    riff: Rc<Riff<T>>,
    #[deref] #[deref_mut] #[index] #[index_mut]
    file_list: StdList<ChunkFile>,
    file_dict: Dict,
}

impl <T: Reader> RiffContainer<T> {
    pub fn new(input: T) -> AResult<Self> {
        let riff = Riff::new(input).context("Bad RIFF container")?;
        let file_list = riff.load_chunk::<StdList<ChunkFile>>(riff.first_of_kind(b"List")).context("Bad List chunk")?;
        let file_dict = riff.load_chunk::<Dict>(riff.first_of_kind(b"Dict")).context("Bad Dict chunk")?;

        Ok(Self {
            riff: Rc::new(riff),
            file_list: Rc::try_unwrap(file_list).unwrap(),
            file_dict: Rc::try_unwrap(file_dict).unwrap(),
        })
    }

    #[must_use]
    pub fn filename(&self, index: usize) -> Option<&BStr> {
        self.file_dict.0.key_by_index(index)
    }

    #[must_use]
    pub fn kind(&self, index: usize) -> Option<ChunkFileKind> {
        self.file_list.get(index).map(|i| i.kind)
    }

    pub fn load_file(&self, index: usize) -> RiffResult<Riff<T>> {
        self.riff.load_riff(self.file_list[index].chunk_index)
    }
}
