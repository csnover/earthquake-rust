use anyhow::{Result as AResult, Context};
use byteorder::{BigEndian, ByteOrder};
use byteordered::Endianness;
use crate::pvec;
use derive_more::{Deref, DerefMut, Index, IndexMut};
use libcommon::{Reader, Resource, resource::{Input, StringContext, StringKind}};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::path::PathBuf;
use super::{List, cast::{MemberId, MemberNum}};

pvec! {
    pub struct CastList {
        #[offset(4..6)]
        field_4: i16,
        #[offset(6..8)]
        count: i16,
        #[offset(8..10)]
        entries_per_cast: i16,
        #[offset(10..12)]
        field_a: i16,
    }
}

#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq)]
pub enum Preload {
    None = 0,
    AfterFirstFrame,
    BeforeFirstFrame,
    Unknown = 4,
}

impl Default for Preload {
    fn default() -> Self {
        Preload::None
    }
}

#[derive(Clone, Debug, Default)]
pub struct Cast {
    name: String,
    path: PathBuf,
    base_resource_num: i32,
    global_cast_id: i16,
    preload: Preload,
    cast_range: (MemberNum, MemberNum),
    is_external_cast: bool,
    is_global_cast_locked: bool,
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
        let mut input = input.as_mut().into_endianness(Endianness::Big);
        Ok(Self(List::<MemberId>::load(&mut input, size, context)?))
    }
}

pub struct CastListIter<'owner> {
    owner: &'owner CastList,
    index: i16,
    count: i16,
    entries_per_cast: i16,
}

impl <'owner> Iterator for CastListIter<'owner> {
    type Item = Cast;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.count {
            None
        } else {
            let base_index = (self.index * self.entries_per_cast) as usize;
            self.index += 1;

            let name = if self.entries_per_cast < 1 {
                String::new()
            } else {
                let context = StringContext(StringKind::PascalStr, self.owner.inner.decoder);
                self.owner.inner.load_entry::<String>(base_index + 1, &context).unwrap_or_default()
            };

            let path = if self.entries_per_cast < 2 || self.index == 1 {
                PathBuf::new()
            } else {
                // TODO: This should be receiving the platform of the Movie
                // that it came from so that it can be parsed into a path
                // correctly.
                let context = StringContext(StringKind::PascalStr, self.owner.inner.decoder);
                PathBuf::from(self.owner.inner.load_entry::<String>(base_index + 2, &context).unwrap_or_default())
            };

            let preload = if self.entries_per_cast < 3 {
                Preload::None
            } else {
                let value = self.owner.inner.load_entry::<i16>(base_index + 3, &()).unwrap_or_default();
                Preload::from_i16(value).with_context(|| format!("Invalid cast preload mode {}", value)).unwrap()
            };

            let (base_resource_num, cast_range) = if self.entries_per_cast < 4 {
                Default::default()
            } else {
                self.owner.inner.load_entry::<Vec<u8>>(base_index + 4, &()).map(|data| {(
                    BigEndian::read_i32(&data[4..]),
                    (BigEndian::read_i16(&data[0..]).into(), BigEndian::read_i16(&data[2..]).into())
                )}).unwrap_or_default()
            };

            Some(Cast {
                name,
                path,
                base_resource_num,
                global_cast_id: 0,
                preload,
                cast_range,
                is_external_cast: self.index != 1,
                is_global_cast_locked: false,
                field_16: false,
            })
        }
    }
}

impl CastList {
    pub fn iter(&self) -> CastListIter<'_> {
        CastListIter {
            owner: self,
            index: 0,
            count: self.count().expect("Bad MCsL: missing count"),
            entries_per_cast: self.entries_per_cast().expect("Bad MCsL: missing entries per cast"),
        }
    }
}
