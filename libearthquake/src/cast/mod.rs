use crate::{collections::riff::Riff, fonts::{Fmap, Map, Source as ExtendedFontMap}, player::movie::ModifiedFlags, resources::{Dict, cast::{CastMetadata, Ccl, Library, MemberNum}, config::{Platform, Version}}};
use libcommon::{Reader, prelude::*};
use libmactoolbox::{resources::ResNum, types::MacString};
use std::{path::PathBuf, rc::Rc};

// RE: CastLibMemberLUT
struct MemberLookup {
    next_free_num: MemberNum,
    next_free_maybe_index: Unk16,
    // Index is cast member number, value is index in library. Library is
    // 1-indexed just to make stuff extra confusing.
    lookup: Vec<usize>,
}

// RE: CastLibLoadContext
struct LoadContext {
    // Source should probably be a dyn ResourceSource since it cannot be a Riff
    // for D3
    source: Option<Rc<Riff<Box<dyn Reader + 'static>>>>,
    list_511a5c_num: i32,
    vwcf_version: Version,
    font_map: Option<Rc<FontMap>>,
}

// RE: CastLibLoadContextFontSizeMap
struct FontSizeMap {
    font_family_id: ResNum,
    // Originally u16
    map_all: bool,
    size_map: Map,
}

// RE: CastLibFontMap
struct FontMap {
    fxmp: Option<Rc<ExtendedFontMap>>,
    fmap: Option<Rc<Fmap>>,
    font_size_maps: Vec<FontSizeMap>,
    current_platform_is_target: bool,
    platform: Platform,
    character_map: Map,
}

// RE: CastLib
struct Cast {
    members: Library,
    cast_num_to_index: MemberLookup,
    load_context: LoadContext,
    own_path: PathBuf,
    /// Original author’s file directory.
    original_path: MacString,
    /// Resolved local file directory.
    local_path: MacString,
    ccl: Option<Rc<Ccl>>,
    // Ref count and lock count were here, should not be needed with std Rc
    /// The index of the cast file if it is embedded in a RiffContainer.
    embedded_file_index: i32,
    cinf: Option<Rc<CastMetadata>>,
    next_free: MemberNum,
    some_purge_156: MemberNum,
    some_purge_158: MemberNum,
    last_cached_name: MemberNum,
    name_cache: Option<Dict<usize>>,
    max_cast_num: MemberNum,
    platform: Platform,
    is_external_cast: bool,
    modified_flags: ModifiedFlags,
}
