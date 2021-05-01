use binrw::BinRead;
use slab::Slab;
use crate::{collections::riff::Riff, fonts::{Fmap, Map, Source as ExtendedFontMap}, player::movie::ModifiedFlags, resources::{Dict, cast::{CastMetadata, Ccl, Library, MemberNum}, config::{Platform, Version}}, util::Path};
use libcommon::{Reader, newtype_num, prelude::*};
use libmactoolbox::{resources::ResNum, types::MacString};
use smart_default::SmartDefault;
use std::rc::Rc;

newtype_num! {
    #[derive(BinRead, Debug)]
    pub(crate) struct GlobalLibNum(i16);
}

#[derive(Debug)]
pub(crate) struct Manager {
    casts: Slab<Cast>,
    platform: Platform,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum SharedCastId<'path> {
    Path(&'path Path),
    Index(i32),
    PathAndIndex(&'path Path, i32),
}

impl Manager {
    pub(crate) fn new(platform: Platform) -> Self {
        Self {
            casts: Slab::with_capacity(1),
            platform
        }
    }

    pub(crate) fn get(&self, index: GlobalLibNum) -> Option<&Cast> {
        self.casts.get(index.unwrap_into())
    }

    pub(crate) fn get_mut(&mut self, index: GlobalLibNum) -> Option<&mut Cast> {
        self.casts.get_mut(index.unwrap_into())
    }

    // RE: `CastLibList_FindNumByPathOrEmbeddedIndex`
    pub(crate) fn get_index(&self, id: SharedCastId<'_>) -> Option<GlobalLibNum> {
        match id {
            SharedCastId::Path(path) => {
                self.casts.iter().find(|cast| &cast.1.own_path == path)
            },
            SharedCastId::Index(index)
            | SharedCastId::PathAndIndex(_, index) => {
                self.casts.iter().find(|cast| cast.1.embedded_file_index == index)
            }
        }.map(|cast| GlobalLibNum::unwrap_from(cast.0))
    }

    // RE: The non-MovieCast parts of `Movie::EnsureGlobalCastLibExists`
    pub(crate) fn get_or_insert_index(&mut self, id: SharedCastId<'_>) -> GlobalLibNum {
        self.get_index(id).unwrap_or_else(|| {
            let mut cast = Cast::new(self.platform);
            match id {
                SharedCastId::Index(index) => cast.embedded_file_index = index,
                SharedCastId::Path(path) => {
                    cast.own_path = path.clone();
                    cast.is_external_cast = true;
                },
                SharedCastId::PathAndIndex(path, index) => {
                    cast.own_path = path.clone();
                    cast.embedded_file_index = index;
                    cast.is_external_cast = true;
                }
            }
            self.casts.insert(cast).unwrap_into()
        })
    }

    pub(crate) fn insert_new(&mut self) -> GlobalLibNum {
        self.casts.insert(Cast::new(self.platform)).unwrap_into()
    }
}

// RE: CastLibMemberLUT
#[derive(Debug, SmartDefault)]
struct MemberLookup {
    /// The next free cast member number in the free list.
    next_free_num: MemberNum,

    /// The highest cast member number.
    #[default(1_i16.into())]
    max_num: MemberNum,

    // Index is cast member number, value is index in library. Library is
    // 1-indexed just to make stuff extra confusing.
    lookup: Vec<usize>,
}

// RE: CastLibLoadContext
#[derive(Debug, Default)]
struct LoadContext {
    // Source should probably be a dyn ResourceSource since it cannot be a Riff
    // for D3
    source: Option<Rc<Riff<Box<dyn Reader + 'static>>>>,
    lingo_environment_num: i32,
    vwcf_version: Version,
    font_map: Option<Rc<FontMap>>,
}

// RE: CastLibLoadContextFontSizeMap
#[derive(Debug, Default)]
struct FontSizeMap {
    font_family_id: ResNum,
    // Originally u16
    map_all: bool,
    size_map: Map,
}

// RE: CastLibFontMap
#[derive(Debug, Default)]
pub(crate) struct FontMap {
    fxmp: Option<Rc<ExtendedFontMap>>,
    fmap: Option<Rc<Fmap>>,
    font_size_maps: Vec<FontSizeMap>,
    current_platform_is_target: bool,
    platform: Platform,
    character_map: Map,
}

// RE: CastLib
#[derive(Debug, SmartDefault)]
pub(crate) struct Cast {
    members: Library,
    cast_num_to_index: MemberLookup,
    load_context: LoadContext,
    own_path: Path,
    /// Original author’s file directory.
    original_path: MacString,
    /// Resolved local file directory.
    local_path: MacString,
    ccl: Option<Rc<Ccl>>,
    #[default(1)]
    ref_count: i32,
    #[default(-1)]
    embedded_file_index: i32,
    cinf: Option<Rc<CastMetadata>>,
    next_free: MemberNum,
    some_purge_156: MemberNum,
    some_purge_158: MemberNum,
    last_cached_name: MemberNum,
    name_cache: Option<Dict<usize>>,
    max_cast_num: MemberNum,
    // TODO: This should default to the ‘current’ platform
    platform: Platform,
    is_external_cast: bool,
    modified_flags: ModifiedFlags,
}

impl Cast {
    fn new(platform: Platform) -> Self {
        Self {
            platform,
            ..<_>::default()
        }
    }

    pub(crate) fn set_font_map(&mut self, font_map: Option<Rc<FontMap>>) {
        self.load_context.font_map = font_map;
    }
}
