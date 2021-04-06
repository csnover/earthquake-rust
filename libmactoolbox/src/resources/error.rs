use binrw::io;
use super::{OsType, RefNum, ResNum, ResourceId};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("bad OSType size")]
    BadOsTypeSize,
    #[error("unknown i/o error: {0}")]
    Io(#[from] io::Error),
    #[error("input borrow failed: {0}")]
    BorrowMutFailed(#[from] core::cell::BorrowMutError),
    #[error("resource {0} not found")]
    NotFound(ResourceId),
    #[error("resource number {1} not found in any {}", .0.iter().map(std::string::ToString::to_string).collect::<Vec<_>>().join(", "))]
    NotFoundNum(&'static [OsType], ResNum),
    #[error("resource {0} uses unsupported compression")]
    UnsupportedCompression(ResourceId),
    #[error("bad data type for resource {0}")]
    BadDataType(ResourceId),
    #[error("i/o error seeking to resource {0}: {1}")]
    SeekFailure(ResourceId, io::Error),
    #[error("i/o error reading size of resource {0}: {1}")]
    ReadSizeFailure(ResourceId, io::Error),
    #[error("i/o error reading header: {0}")]
    HeaderReadIo(io::Error),
    #[error("i/o error reading map size: {0}")]
    MapSizeReadIo(io::Error),
    #[error("bad fork data size ({0})")]
    BadDataSize(u32),
    #[error("bad map offset ({0})")]
    BadMapOffset(u32),
    #[error("bad map size ({0})")]
    BadMapSize(u32),
    #[error("bad map OSType count ({0})")]
    BadMapKindCount(i16),
    #[error("bad map resource count ({0}) for OSType {1}")]
    BadMapResourceCount(i16, OsType),
    #[error("can’t decompress resource {0}: {1}")]
    BadCompression(ResourceId, io::Error),
    #[error("file too small ({0} < {1})")]
    FileTooSmall(u64, u64),
    #[error("bad resource map")]
    BadResourceMap,
    #[error("can’t find Application VISE signature on resource {0}")]
    MissingViseSignature(ResourceId),
    #[error("can’t find Application VISE CODE resource")]
    MissingViseResource,
    #[error("can’t find Application VISE shared dictionary")]
    MissingViseDictionary,
    #[error("missing decompressor")]
    MissingDecompressor,
    #[error("invalid resource file number {0}")]
    BadRefNum(RefNum),
    #[error("current_file invalid ({0} >= {1})")]
    BadCurrentFile(usize, usize),
    #[error("no system file")]
    NoSystemFile,
    #[error("vfs error: {0}")]
    VfsFailure(libcommon::vfs::Error),
    #[error("error reading {0}: {1}")]
    ResourceReadFailure(ResourceId, binrw::Error),
    #[error("can’t create system resource from memory: {0}")]
    BadSystemResource(Box<Self>),
}

impl From<binrw::Error> for Error {
    fn from(error: binrw::Error) -> Self {
        match error {
            binrw::Error::Io(error) => Self::Io(error),
            binrw::Error::Custom { err, .. } => {
                *err.downcast().expect("unexpected error type")
            },
            _ => panic!("unexpected error type: {}", error),
        }
    }
}
