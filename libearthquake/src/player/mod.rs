pub(super) mod movie;
pub(super) mod score;
mod window;

use anyhow::{Context, Result as AResult, bail};
use binrw::io::SeekFrom;
use crate::{cast::Manager as CastManager, collections::{riff::Riff, riff_container::RiffContainer}, detection::{Detection, FileType, Version, projector::{D3WinMovie, Movie as ProjectorMovie}}, event::Manager as EventManager, resources::{cast::MemberId, config::Platform}, sound::Manager as SoundManager, util::Path};
use libcommon::{prelude::*, vfs::{VirtualFile, VirtualFileSystem}};
use libmactoolbox::{System, events::{EventData, EventKind}, intl::ScriptCode, quickdraw::Pixels, resources::File as ResourceFile, types::MacString};
use self::{movie::{Manager as MovieManager, ModifiedFlags, MovieListNum}, score::Palette};
use std::rc::Rc;

#[derive(Debug)]
enum Playlist<'vfs> {
    RiffContainer(RiffContainer<Box<dyn VirtualFile + 'vfs>>),
    SingleRiff(Riff<Box<dyn VirtualFile + 'vfs>>),
    D3Win(Vec<D3WinMovie>),
    D3Mac(Vec<MacString>),
    Embeds(ResourceFile<Box<dyn VirtualFile + 'vfs>>, u16),
}

impl <'vfs> Playlist<'vfs> {
    fn len(&self) -> usize {
        match self {
            Playlist::RiffContainer(c) => c.len(),
            Playlist::SingleRiff(_) => 1,
            Playlist::D3Win(l) => l.len(),
            Playlist::D3Mac(l) => l.len(),
            Playlist::Embeds(_, c) => usize::from(*c)
        }
    }
}

#[derive(Clone, Debug, Default)]
struct PaletteManagerState {
    mode: Unk16,
    num_frames: i16,
    ct_size: i16,
    ct_handle_byte_size: u32,
    // TODO: These are actually handles to ColorTables, but it will be a while
    // before that code exists.
    fade_table_maybe: UnkHnd,
    pm_table_2: UnkHnd,
    pm_table_3: UnkHnd,
    is_fade_maybe_ping_pong_maybe: bool,
    at_cycle_wrap_maybe: bool,
    has_realloc_error: bool,
    has_32_bit_qd: bool,
    score_palette: Palette,
    cycle_start_index: i16,
    cycle_end_index: i16,
    g_devices: [ UnkHnd; 6 ],
    num_g_devices: i16,
}

#[derive(Clone, Debug, Default)]
struct PaletteManager {
    field_0: Unk32,
    state: PaletteManagerState,
    id: MemberId,
    the_gdevice_palette_seed_is_set: bool,
    field_4_gdevice_palettes_set: bool,
    gdevices_in_port_rect_bitmap: bool,
    num_frames_left: i16,
    field_12: Unk8,
    dont_set_the_gdevice_palette_seed: bool,

    // These values are normally separate globals in OD (probably because they
    // are Windows-specific), but they are related to palette management, so are
    // included here
    animation: bool,
    patch_windows_colors: bool,
}

struct Lingo;

struct MacResIdMap;

pub struct Player<'vfs> {
    system: System<'vfs>,
    // RE: `g_player_projectorPath`
    source_path: Path,
    playlist: Playlist<'vfs>,
    // RE: `g_event`
    event: EventManager,
    // RE: `g_sound`
    sound: SoundManager,
    // RE: `g_paletteManager`
    palette: PaletteManager,
    // RE: `g_lingo`
    lingo: Lingo,

    // RE: `g_activeMovieList`
    movies: MovieManager,
    // RE: `g_castLibList`
    casts: CastManager,

    // RE: `g_VWTLSizeLUT`
    tile_sizes: [ Pixels; 5 ],

    // RE: `g_movie`
    // OD had a global movie object and a global pointer to whatever movie was
    // being operated on. The pointer defaulted to point to this global object
    // and was swapped when the engine had to operate on other movies. OD also
    // had a list of pointers to active (i.e. not preloaded) movies
    // (`g_activeMovieList`) which was used mostly for memory management. In
    // this implementation, the movie list is used to (try to; we’ll see how it
    // goes) avoid using `Rc<RefCell<Movie>>` by using a `MovieListNum` in areas
    // where the original code held mutable pointers to movies instead.
    // There was also a global score object, used to swap around and play film
    // loop resources, but again hopefully we are going to actually be passing
    // objects instead of relying on globals in this implementation so that is
    // elided.
    movie: MovieListNum,

    // RE: `g_idleLoadList`
    idle_load_list: MacResIdMap,

    // RE: `g_movie_modifiedFlags`
    modified_flags: ModifiedFlags,

    // This was normally baked in to the projector everywhere, but since this
    // one is actually universal, it must be defined.
    platform: Platform
}

impl <'vfs> Player<'vfs> {
    // RE: `Player_Init`
    pub fn new(vfs: Rc<dyn VirtualFileSystem + 'vfs>, charset: Option<ScriptCode>, path: String, source: Detection<'vfs>) -> AResult<Self> {
        // RE: `OVWD_InitWorld` {

        // TODO: OVWDWorld_Init
        // TODO: OVWD_InitSystem - Set original projector path

        // OD: A whole lot of irrelevant legacy stuff is elided here, like
        // making sure the Mac OS version is new enough, checking whether it has
        // support for Color QuickDraw, true colour graphics, setting up window
        // management since apparently you had to do that yourself on Classic
        // Mac OS, etc. Also kept out a whole lot of unnecessary work which was
        // probably unnecessary even in OD, and initialisations which instead
        // happen using the magic of modern programming languages that have cool
        // things like default constructors.

        // RE: `OVWD_InitEngine` {
        // RE: `OVWD_InitEngineImpl` {

        // OD: More irrelevant stuff defaulted by constructors elided here.

        let (platform, detected_charset, system_data, playlist) = detection_to_playlist(source)?;
        let charset = charset.or(detected_charset).unwrap_or(ScriptCode::Roman);

        let this = Self {
            platform,
            source_path: Path::new(path, platform),
            playlist,
            event: EventManager::new(),
            // RE: `Sound_Init`
            sound: SoundManager::new(),
            // TODO: `PaletteManager_New`
            palette: <_>::default(),
            movies: MovieManager::new(),
            casts: CastManager::new(platform),
            movie: <_>::default(),
            // RE: `OVWD_InitMovieVWTL`
            tile_sizes: [
                0_i16.into(),
                0x10_i16.into(),
                0x20_i16.into(),
                0x40_i16.into(),
                0x80_i16.into()
            ],
            // TODO: `OVWD_InitLingo`
            lingo: Lingo,
            idle_load_list: MacResIdMap,
            system: System::new(vfs, charset, system_data)?,
            modified_flags: <_>::default(),
        };

        // TODO: `OVWD_InitTextRescale?LUT`
        // TODO: `OVWD_InitPrint` (if anyone cares)
        // TODO: Set RIFF sound servicing proc (if actually needed, hello
        // modern software with threading)
        // TODO: `OVWD_InitXtras`
        // TODO: `Lingo_LoadIni`

        // RE: } `OVWD_InitEngineImpl`

        // TODO: `Score_InitEditableSprite`

        // RE: } `OVWD_InitEngine`

        // TODO: For some reason, code here tries to set the score’s editable
        // sprite rect to the gray region. Why? Is this necessary?

        // RE: } `OVWD_InitWorld`

        // RE: `OVWD_ConfigureOVWDWorld`

        // TODO: The settings set here were to play in background and to wait
        // for click.

        // RE: } `OVWD_ConfigureOVWDWorld`

        Ok(this)
    }

    // RE: `Player_LoadFirstMovie` + first call to `Player_PlayNextMovie`?
    pub fn exec(&mut self) -> AResult<()> {
        todo!()
    }

    // RE: `Event_Handle`
    pub fn post_event(&mut self, _kind: EventKind, _data: EventData) -> AResult<()> {
        todo!()
    }

    #[must_use]
    pub(crate) fn platform(&self) -> Platform {
        self.platform
    }

    #[must_use]
    pub(crate) fn modified_flags(&self) -> ModifiedFlags {
        self.modified_flags
    }

    pub(crate) fn set_modified_flags(&mut self, flags: ModifiedFlags) -> ModifiedFlags {
        let old_flags = self.modified_flags;
        self.modified_flags = flags;
        old_flags
    }
}

fn detection_to_playlist(mut file: Detection<'_>) -> AResult<(Platform, Option<ScriptCode>, Option<Vec<u8>>, Playlist<'_>)> {
    Ok(match file.info() {
        FileType::Projector(p) => (
            p.config().platform().into(),
            p.charset(),
            p.system_resources().cloned(),
            match p.movie() {
                &ProjectorMovie::Embedded(count) => {
                    if let Some(resource_fork) = file.resource_fork.take() {
                        let resource_file = ResourceFile::new(resource_fork)
                            .context("Can’t create resource file for projector")?;
                        Playlist::Embeds(resource_file, count)
                    } else {
                        bail!("Missing resource fork for projector");
                    }
                },
                ProjectorMovie::D3Win(movies) => Playlist::D3Win(movies.clone()),
                &ProjectorMovie::Internal(offset) => Playlist::RiffContainer({
                    if let Some(mut input) = file.data_fork.take() {
                        input.seek(SeekFrom::Start(offset.into())).context("Can’t seek to RIFF container")?;
                        RiffContainer::new(input).context("Can’t create RIFF container from data fork")?
                    } else {
                        bail!("Missing data fork for RIFF container");
                    }
                }),
                ProjectorMovie::External(files) => Playlist::D3Mac(files.clone()),
            },
        ),
        FileType::Movie(m) => (
            m.platform(),
            None,
            None,
            if m.version() == Version::D3 {
                if let Some(resource_fork) = file.resource_fork.take() {
                    let resource_file = ResourceFile::new(resource_fork)
                        .context("Can’t create resource file for movie")?;
                    Playlist::Embeds(resource_file, 1)
                } else {
                    bail!("Missing resource fork for movie");
                }
            } else {
                Playlist::SingleRiff(
                    Riff::new(
                        file.data_fork.take().context("Missing data fork for movie")?
                    ).context("Can’t create RIFF from data fork")?
                )
            }
        ),
    })
}
