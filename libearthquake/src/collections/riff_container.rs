use anyhow::{anyhow, Context, Result as AResult};
use binrw::{BinRead, io::{Read, Seek}};
use bstr::BStr;
use crate::resources::{SerializedDict, StdList};
use derive_more::{Deref, DerefMut};
use libcommon::{Reader, SeekExt, restore_on_error};
use libmactoolbox::typed_resource;
use smart_default::SmartDefault;
use std::rc::Rc;
use super::riff::{ChunkIndex, Riff, Result as RiffResult};

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

#[derive(BinRead, Clone, Debug, Deref, DerefMut)]
pub struct Dict(SerializedDict<usize>);
typed_resource!(Dict => b"Dict");

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
#[derive(Clone, Debug, Deref, DerefMut)]
pub struct RiffContainer<T: Reader> {
    riff: Rc<Riff<T>>,
    #[deref] #[deref_mut]
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
            file_list: Rc::try_unwrap(file_list).map_err(|_| anyhow!("Can’t unwrap RiffContainer list"))?,
            file_dict: Rc::try_unwrap(file_dict).map_err(|_| anyhow!("Can’t unwrap RiffContainer dict"))?,
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
