use binrw::{BinRead, io};
use crate::{pvec, util::RawString};
use derive_more::{Deref, DerefMut};
use libcommon::{prelude::*, bitflags, restore_on_error};
use libmactoolbox::{typed_resource, types::PString};
use super::{PVecOffsets, StdList, cast::{MemberId, MemberNum}};
use smart_default::SmartDefault;

bitflags! {
    /// The kinds of event handlers registered in the movie script.
    struct EventHandlers: u32 {
        const MOUSE_UP    = 1;
        const MOUSE_DOWN  = 2;
        const IDLE        = 4;
        const START_MOVIE = 8;
        const STOP_MOVIE  = 0x10;
        const STEP_MOVIE  = 0x20;
    }
}

bitflags! {
    struct FileInfoFlags: u32 {
        const WINDOW_RELATED_MAYBE = 1;
        const INIT_DEFAULT_MAYBE   = 0x20;
        const REMAP_PALETTES       = 0x40;
        const DIRECTOR_4_RELATED   = 0x80;
        const MOVIE_FIELD_47       = 0x100;
        const UPDATE_MOVIE_ENABLED = 0x200;
        const PRELOAD_EVENT_ABORT  = 0x400;
        const MOVIE_FIELD_4D       = 0x1000;
        const MOVIE_FIELD_4E       = 0x2000;
    }
}

pvec! {
    /// Information about a movie file.
    ///
    /// OsType: `'VWFI'`
    #[derive(Debug)]
    pub struct FileInfo {
        header_size = header_size;

        header {
            script_handle: UnkHnd,
            /// The kinds of event handlers in the movie script.
            ///
            /// Not used by D4+, which store the movie script in
            /// `'Lctx'`/`'Lscr'` chunks.
            event_handlers: EventHandlers,
            flags: FileInfoFlags,
            #[br(if(header_size >= 20))]
            script_context_num: u32,
        }

        offsets = offsets;

        entries {
            /// The movie script.
            ///
            /// Not used by D4+, which store the movie script in
            /// `'Lctx'`/`'Lscr'` chunks.
            #[br(count(offsets.entry_size(0).unwrap_or(0)))]
            0 => movie_script_text: RawString,
            /// The name of the user who created the file.
            1 => creator_name: PString,
            /// The name of the user who last modified the file.
            2 => modify_name: PString,
            /// The original path of the file, excluding the filename.
            3 => path: PString,
            4 => flag_20_related: i16,
            5 => entry_5: i16,
            6 => entry_6: i16,
            7 => entry_7: i16,
            8..
        }
    }
}
typed_resource!(FileInfo => b"VWFI");

pvec! {
    /// The list of cast libraries used by a movie.
    ///
    /// OsType: `'MCsL'`
    /// RE: `MovieCastList`
    #[derive(Debug)]
    pub struct CastList {
        header_size = header_size;

        header {
            field_4: i16,
            count: i16,
            entries_per_cast: i16,
            field_a: i16,
        }

        offsets = offsets;

        entries {
            #[br(args(entries_per_cast, offsets.clone()), count(count))]
            _  => members: CastListMembers,
        }
    }
}
typed_resource!(CastList => b"MCsL");

#[derive(Debug)]
pub struct CastListMembers(Vec<Cast>);

impl BinRead for CastListMembers {
    type Args = (i16, PVecOffsets);

    fn read_options<R: io::Read + io::Seek>(reader: &mut R, options: &binrw::ReadOptions, (entries_per_cast, offsets): Self::Args) -> binrw::BinResult<Self> {
        if let Some(count) = options.count {
            restore_on_error(reader, |reader, _| {
                let mut options = *options;
                options.endian = binrw::Endian::Big;

                let mut data = Vec::with_capacity(count);
                let entries_per_cast = usize::unwrap_from(entries_per_cast);
                for index in 0..count {
                    // `+ 1` because whoever wrote this feature decided that
                    // 1-indexing was the way to go
                    let cast_offsets = offsets.slice(1 + index * entries_per_cast, entries_per_cast);
                    data.push(Cast::read_options(reader, &options, (index, cast_offsets))?);
                }
                Ok(Self(data))
            })
        } else {
            Ok(Self(Vec::new()))
        }
    }
}

#[derive(BinRead, Clone, Copy, Debug, Eq, PartialEq, SmartDefault)]
#[br(big, repr(i16))]
pub enum Preload {
    #[default]
    None = 0,
    AfterFirstFrame,
    BeforeFirstFrame,
    Unknown = 4,
}

/// Movie cast library metadata.
///
/// Starting in Director 5, multiple cast libraries were allowed per movie.
/// Earlier versions of Director had a single cast per movie plus a single
/// optional shared cast named “Shared Cast” (D3Mac), “SHRDCST.DIR” (D3Win),
/// or “SHARED.DIR” (D4).
///
/// In order to keep things spicy, the on-disk representation of this object is
/// not the same as the original in-memory representation. In memory, the order
/// of fields is normally:
///
/// `name`, `path`, `base_resource_num`, `global_cast_id`, `preload`,
/// `min_cast_num`, `max_cast_num`, `is_external`, `is_global_cast_locked`,
/// `field_16`, <padding byte>.
///
/// RE: `MovieCast`
#[derive(BinRead, Clone, Debug, Default)]
#[br(big, import(index: usize, offsets: PVecOffsets))]
pub struct Cast {
    /// The user-defined name of the cast.
    // TODO: In `Movie::ParseMCsL` this is mapped from Mac codepage to the
    // Windows codepage.
    #[br(if(offsets.has_entry(0)), pad_size_to = offsets.entry_size(0).unwrap_or(0))]
    name: PString,
    /// The path of the cast, as a string.
    // TODO: In `Movie::ParseMCsL` this ran extra steps if the entry existed:
    // 1. Swap the movie global with the passed movie object
    // 2. String mapped from Mac codepage to Windows codepage
    // 3. Check to see if path exists within the global `RiffContainer`; if yes,
    //    use as-is. If not, call to `Movie_ResolveCastPathByName`, and replace
    //    the path with the one from the user, and set the movie’s
    //    `modifiedFlags` field from the `g_movie_modifiedFlags` global if the
    //    user had to manually give a path to the file.
    // 4. Restore the original movie.
    #[br(if(offsets.has_entry(1) && index != 0), pad_size_to = offsets.entry_size(1).unwrap_or(0))]
    path: PString,
    /// The mode to use when preloading cast members.
    #[br(if(offsets.has_entry(2)), pad_size_to = offsets.entry_size(2).unwrap_or(0))]
    preload: Preload,
    /// The minimum and maximum cast member numbers in this cast.
    #[br(if(offsets.has_entry(3)))]
    cast_range: (MemberNum, MemberNum),
    /// The Mac resource number assigned to this cast library.
    #[br(if(offsets.has_entry(3)), pad_size_to = offsets.entry_size(3).unwrap_or(8) - 8)]
    base_resource_num: i32,
    /// The 1-indexed number of this library in the global cast list.
    #[br(default)]
    global_cast_id: i16,
    /// If true, this cast is loaded from an external file.
    #[br(calc(path != ""))]
    is_external_cast: bool,
    #[br(default)]
    is_global_cast_locked: bool,
    #[br(default)]
    field_16: bool,
}

/// The list of all cast members in the movie, sorted by the order in which they
/// first appear in the score.
///
/// The intent of this list is to enable preloading of cast members
/// before/during playback.
///
/// Internal cast members which are not in the score are included at the end of
/// the list.
///
/// `OsType`: `'Sord'`
#[derive(BinRead, Clone, Debug, Deref, DerefMut)]
#[br(big)]
pub struct CastScoreOrder(StdList<MemberId>);
