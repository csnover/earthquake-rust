use binrw::{BinRead, io};
use derive_more::{Deref, DerefMut};
use libcommon::{SeekExt, restore_on_error};
use libmactoolbox::{quickdraw::{PixPatHandle, Rect}, typed_resource};
use super::cast::{MemberId, MemberNum};

#[derive(BinRead, Clone, Copy, Debug, Default)]
#[br(big)]
struct TileV3 {
    #[br(default, pad_after = 4)]
    pix_pat: Option<PixPatHandle>,
    id: MemberNum,
    rect: Rect,
}

impl TileV3 {
    const SIZE: u64 = 14;
}

impl From<TileV3> for TileV5 {
    fn from(old: TileV3) -> Self {
        Self {
            pix_pat: old.pix_pat,
            id: old.id.into(),
            rect: old.rect,
        }
    }
}

#[derive(BinRead, Clone, Copy, Debug, Default)]
#[br(big)]
pub struct TileV5 {
    #[br(default, pad_after = 4)]
    pix_pat: Option<PixPatHandle>,
    id: MemberId,
    rect: Rect,
}

impl TileV5 {
    const SIZE: u64 = 16;
}

#[derive(BinRead, Clone, Copy, Debug, Default, Deref, DerefMut)]
#[br(big)]
pub struct TilesV5([ TileV5; 8 ]);

/// A set of tiles used for drawing patterns.
///
/// OsType: `'VWTL'`
#[derive(Clone, Copy, Debug, Default, Deref, DerefMut)]
pub struct Tiles(TilesV5);
typed_resource!(Tiles => b"VWTL");

impl Tiles {
    const SIZE_V3: u64 = TileV3::SIZE * 8;
    const SIZE_V5: u64 = TileV5::SIZE * 8;
}

impl BinRead for Tiles {
    type Args = ();

    fn read_options<R: io::Read + io::Seek>(
        reader: &mut R,
        _: &binrw::ReadOptions,
        _: Self::Args,
    ) -> binrw::BinResult<Self> {
        restore_on_error(reader, |reader, _| {
            let size = reader.bytes_left()?;

            if size != Self::SIZE_V3 && size != Self::SIZE_V5 {
                let pos = reader.pos()?;
                return Err(binrw::Error::AssertFail {
                    pos,
                    message: format!("unexpected VWTL size {}", size)
                });
            }

            let mut tiles = Self::default();
            for tile in tiles.iter_mut() {
                if size == Self::SIZE_V3 {
                    *tile = TileV3::read(reader)?.into();
                } else {
                    *tile = TileV5::read(reader)?;
                }
            }
            Ok(tiles)
        })
    }
}
