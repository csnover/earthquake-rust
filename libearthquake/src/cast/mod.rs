use crate::{collections::riff::Riff, fonts::{Fmap, Map, Source as ExtendedFontMap}, player::movie::ModifiedFlags, resources::{Dict, cast::{CastMetadata, Ccl, Library, MemberNum}, config::{Platform, Version}}};
use libcommon::prelude::*;
use libmactoolbox::{resources::ResNum, types::MacString};
use std::{path::PathBuf, rc::Rc};

struct List51b698ItemLut {
    next_free_num: MemberNum,
    next_free_maybe_index: Unk16,
    // Index is cast member number, value is index in library. Library is
    // 1-indexed just to make stuff extra confusing.
    lookup: Vec<usize>,
}

struct List51b698ItemC {
    // Source should probably be a dyn ResourceSource since it cannot be a Riff
    // for D3
//    source: Option<Rc<Riff>>,
    list_511a5c_num: i32,
    vwcf_version: Version,
    font_map: Option<Rc<List51b698ItemCFontMap>>,
}

struct List51b698ItemCFontSizeMap {
    font_family_id: ResNum,
    // Originally u16
    map_all: bool,
    size_map: Map,
}

struct List51b698ItemCFontMap {
    fxmp: Option<Rc<ExtendedFontMap>>,
    fmap: Option<Rc<Fmap>>,
    font_size_maps: Vec<List51b698ItemCFontSizeMap>,
    current_platform_is_target: bool,
    platform: Platform,
    character_map: Map,
}

struct List51b698Item {
    members: Library,
    cast_num_to_index: List51b698ItemLut,
    field_c: List51b698ItemC,
    own_path: PathBuf,
    /// Original author’s file directory.
    original_path: MacString,
    /// Resolved local file directory.
    local_path: MacString,
    ccl: Option<Rc<Ccl>>,
    field_13c: Unk32,
    // Ref count and lock count were here, should not be needed with std Rc
    /// The index of the file if it is embedded in a RiffContainer.
    embedded_file_index: i32,
    field_14c: Unk32,
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
