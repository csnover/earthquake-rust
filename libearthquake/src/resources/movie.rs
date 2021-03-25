use anyhow::{Result as AResult};
use binrw::BinRead;
use libmactoolbox::types::PString;
use crate::pvec;
use derive_more::{Deref, DerefMut, Index, IndexMut};
use libcommon::{Reader, Resource, TakeSeekExt, resource::Input, restore_on_error};
use super::{List, cast::{MemberId, MemberNum}};
use smart_default::SmartDefault;

// MCsL
pvec! {
    #[derive(Debug)]
    pub struct CastList {
        header {
            field_4: i16,
            count: i16,
            entries_per_cast: i16,
            field_a: i16,
        }

        offsets = offsets;

        entries {
            #[br(args(entries_per_cast), count(count))]
            _  => members: CastListMembers,
        }
    }
}

impl CastList {
    pub fn iter(&self) -> impl Iterator<Item = &Cast> {
        self.members.0.iter()
    }
}

#[derive(Debug)]
pub struct CastListMembers(Vec<Cast>);

impl BinRead for CastListMembers {
    type Args = (i16, );

    fn read_options<R: std::io::Read + std::io::Seek>(reader: &mut R, options: &binrw::ReadOptions, (entries_per_cast, ): Self::Args) -> binrw::BinResult<Self> {
        if let Some(count) = options.count {
            restore_on_error(reader, |reader, _| {
                let mut data = Vec::with_capacity(count);
                for index in 0..count {
                    data.push(Cast::read_options(reader, options, (index, entries_per_cast))?);
                }
                Ok(Self(data))
            })
        } else {
            Ok(Self(Vec::new()))
        }
    }
}

#[derive(BinRead, Clone, Copy, Debug, Eq, PartialEq, SmartDefault)]
#[br(repr(i16))]
pub enum Preload {
    #[default]
    None = 0,
    AfterFirstFrame,
    BeforeFirstFrame,
    Unknown = 4,
}

#[derive(BinRead, Clone, Debug, Default)]
#[br(import(index: usize, entries_per_cast: i16))]
pub struct Cast {
    #[br(if(entries_per_cast > 0))]
    name: PString,
    #[br(if(entries_per_cast > 1 && index != 0))]
    path: PString,
    #[br(if(entries_per_cast > 2))]
    preload: Preload,
    // the next two fields are part of the same entry
    #[br(if(entries_per_cast > 3))]
    cast_range: (MemberNum, MemberNum),
    #[br(if(entries_per_cast > 3))]
    base_resource_num: i32,
    #[br(default)]
    global_cast_id: i16,
    #[br(calc(index != 0))]
    is_external_cast: bool,
    #[br(default)]
    is_global_cast_locked: bool,
    #[br(default)]
    field_16: bool,
}

// The list of all cast members in the movie, sorted by the order in which they
// first appear in the score. Internal cast members which are not in the score
// are included at the end of the list.
#[derive(Clone, Debug, Default, Deref, DerefMut, Index, IndexMut)]
pub struct CastScoreOrder(List<MemberId>);
impl Resource for CastScoreOrder {
    type Context = ();

    fn load(input: &mut Input<impl Reader>, size: u32, context: &Self::Context) -> AResult<Self> where Self: Sized {
        let mut options = binrw::ReadOptions::default();
        options.endian = binrw::Endian::Big;
        Ok(Self(List::<MemberId>::read_options(&mut input.take_seek(size.into()), &options, ())?))
    }
}
