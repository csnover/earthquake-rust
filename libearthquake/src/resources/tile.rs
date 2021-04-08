use binrw::{BinRead, io};
use derive_more::{Deref, DerefMut};
use libcommon::{SeekExt, restore_on_error};
use libmactoolbox::{quickdraw::{PixPatHandle, Rect}, typed_resource};
use super::cast::MemberId;

#[derive(BinRead, Clone, Copy, Debug, Default)]
#[br(big, import(is_v5: bool))]
pub struct Tile {
    #[br(default, pad_after = 4)]
    pix_pat: Option<PixPatHandle>,
    #[br(args(is_v5), parse_with = MemberId::parse_num)]
    id: MemberId,
    rect: Rect,
}

/// A set of tiles used for drawing patterns.
///
/// OsType: `'VWTL'`
#[derive(Clone, Copy, Debug, Default, Deref, DerefMut)]
pub struct Tiles([ Tile; 8 ]);
typed_resource!(Tiles => b"VWTL");

impl Tiles {
    const SIZE_V3: u64 = 14 * 8;
    const SIZE_V5: u64 = 16 * 8;
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
            let is_v5 = size == Self::SIZE_V5;
            for tile in tiles.iter_mut() {
                *tile = Tile::read_args(reader, (is_v5, ))?;
            }
            Ok(tiles)
        })
    }
}
