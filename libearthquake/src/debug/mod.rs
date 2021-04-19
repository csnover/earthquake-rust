use anyhow::Result as AResult;
use crate::{collections::{
        riff::Riff,
        riff_container::{ChunkFileKind, RiffContainer},
    }, detection::{
        detect,
        Detection,
        FileType,
        movie::{
            DetectionInfo as MovieDetectionInfo,
            Kind as MovieKind,
        },
        projector::{
            DetectionInfo as ProjectorDetectionInfo,
            Movie as MovieInfo,
        },
        Version,
    }, player::score::Frame, player::score::Score, resources::{cast::Library, config::{Config, Version as ConfigVersion}, movie::{CastList, FileInfo}}};
use libcommon::{io::prelude::*, prelude::*};
use libmactoolbox::{resources::{File as ResourceFile, ResourceId, Source as ResourceSource}, vfs::HostFileSystem};
use std::path::PathBuf;

pub use crate::resources::cast::MemberId;

pub enum Command {
    Detect(bool),
    List(bool),
    PrintCasts,
    PrintCastMember(Vec<MemberId>),
    PrintCastMembers,
    PrintConfig,
    PrintFileInfo,
    PrintResource(Vec<ResourceId>),
    PrintResources,
    PrintScore(i16, Option<(i16, i16)>, Option<Vec<String>>),
}

pub struct Options {
    pub command: Command,
    pub data_dir: Option<PathBuf>,
}

pub type PrintScoreOptions = (i16, Option<(i16, i16)>, Option<Vec<String>>);

impl Options {
    fn detect(&self) -> bool {
        matches!(self.command, Command::Detect(_))
    }

    fn list(&self) -> bool {
        matches!(self.command, Command::List(_))
    }

    fn print_casts(&self) -> bool {
        matches!(self.command, Command::PrintCasts)
    }

    fn print_cast_member(&self) -> Option<&Vec<MemberId>> {
        match self.command {
            Command::PrintCastMember(ref members) => Some(members),
            _ => None,
        }
    }

    fn print_cast_members(&self) -> bool {
        matches!(self.command, Command::PrintCastMembers)
    }

    fn print_config(&self) -> bool {
        matches!(self.command, Command::PrintConfig)
    }

    fn print_file_info(&self) -> bool {
        matches!(self.command, Command::PrintFileInfo)
    }

    fn print_resource(&self) -> Option<&Vec<ResourceId>> {
        match self.command {
            Command::PrintResource(ref resources) => Some(resources),
            _ => None,
        }
    }

    fn print_resources(&self) -> bool {
        matches!(self.command, Command::PrintResources)
    }

    fn print_score(&self) -> Option<PrintScoreOptions> {
        match self.command {
            Command::PrintScore(score_num, frames, ref fields) => Some((score_num, frames, fields.clone())),
            _ => None,
        }
    }

    fn recursive(&self) -> bool {
        match self.command {
            Command::Detect(recursive) | Command::List(recursive) => recursive,
            _ => true,
        }
    }
}

pub fn read_file(filename: &str, options: &Options) -> AResult<()> {
    let fs = HostFileSystem::new();
    let Detection { info, resource_fork, data_fork } = detect(&fs, filename)?;

    if options.detect() {
        println!("{:#?}", info);
    }

    match info {
        FileType::Projector(p) => read_projector(
            &p,
            if p.version() == Version::D3 {
                resource_fork.or(data_fork)
            } else {
                data_fork
            }.unwrap(),
            options
        ),
        FileType::Movie(m) => read_movie(&m, resource_fork.or(data_fork).unwrap(), options),
    }
}

fn print_cast_library(cast: &Library, min_cast_num: i16, options: &Options) {
    for (i, member) in cast.iter().enumerate() {
        let cast_member_num = min_cast_num + i16::unwrap_from(i);
        if options.print_cast_members() || options.print_cast_member().unwrap().contains(&MemberId::new(0_i16, cast_member_num)) {
            println!("{}: {:#?}", cast_member_num, member);
        }
    }
}

fn print_frame(frame: &Frame, fields: &[String]) -> bool {
    let mut print_sprites = false;
    for field in fields.iter() {
        match field.as_str() {
            "script" => println!("Script: {:?}", frame.script),
            "sound_1" => println!("Sound 1: {:?}", frame.sound_1),
            "sound_2" => println!("Sound 2: {:?}", frame.sound_2),
            "transition" => println!("Transition: {:?}", frame.transition),
            "tempo_related" => println!("Tempo related: {:?}", frame.tempo_related),
            "sound_1_related" => println!("Sound 1 related: {:?}", frame.sound_1_related),
            "sound_2_related" => println!("Sound 2 related: {:?}", frame.sound_2_related),
            "script_related" => println!("Script related: {:?}", frame.script_related),
            "transition_related" => println!("Transition related: {:?}", frame.transition_related),
            "tempo" => println!("Tempo: {:?}", frame.tempo),
            "palette" => println!("Palette: {:?}", frame.palette),
            field if field.starts_with("sprites.") => { print_sprites = true; },
            field => eprintln!("Unknown score frame field '{}'", field)
        }
    }
    print_sprites
}

fn print_frame_sprites(frame: &Frame, fields: &[String]) {
    for (i, sprite) in frame.sprites.as_ref().iter().enumerate() {
        for field in fields.iter() {
            match field.as_str() {
                "sprites.kind" => println!("Sprite {} kind: {:?}", i + 1, sprite.kind()),
                "sprites.ink" => println!("Sprite {} ink: {:?}", i + 1, sprite.ink()),
                "sprites.moveable" => println!("Sprite {} moveable: {:?}", i + 1, sprite.moveable()),
                "sprites.editable" => println!("Sprite {} editable: {:?}", i + 1, sprite.editable()),
                "sprites.trails" => println!("Sprite {} trails: {:?}", i + 1, sprite.trails()),
                "sprites.stretch" => println!("Sprite {} stretch: {:?}", i + 1, sprite.stretch()),
                "sprites.blend" => println!("Sprite {} blend: {:?}", i + 1, sprite.blend()),
                "sprites.id" => println!("Sprite {} id: {:?}", i + 1, sprite.id()),
                "sprites.script" => println!("Sprite {} script: {:?}", i + 1, sprite.script()),
                "sprites.fore_color_index" => println!("Sprite {} fore color index: {:?}", i + 1, sprite.fore_color_index()),
                "sprites.back_color_index" => println!("Sprite {} back color index: {:?}", i + 1, sprite.back_color_index()),
                "sprites.origin" => println!("Sprite {} origin: {:?}", i + 1, sprite.origin()),
                "sprites.height" => println!("Sprite {} height: {:?}", i + 1, sprite.height()),
                "sprites.width" => println!("Sprite {} width: {:?}", i + 1, sprite.width()),
                "sprites.score_color" => println!("Sprite {} score color: {:?}", i + 1, sprite.score_color()),
                "sprites.blend_amount" => println!("Sprite {} blend amount: {:?}", i + 1, sprite.blend_amount()),
                "sprites.line_size" => println!("Sprite {} line size: {:?}", i + 1, sprite.line_size()),
                field if field.starts_with("sprites.") => eprintln!("Unknown score frame field '{}'", field),
                _ => {},
            }
        }
    }
}

fn print_mac_resource(rom: &ResourceFile<impl Reader>, id: ResourceId) {
    print_resource(id, rom.load::<Vec<u8>>(id).map_err(anyhow::Error::new));
}

fn print_resource(id: ResourceId, result: AResult<impl std::fmt::Debug>) {
    match result {
        Ok(r) => println!("{}: {:02x?}", id, r),
        Err(e) => eprintln!("Can’t inspect {}: {}", id, e)
    }
}

fn print_riff_resource(riff: &Riff<impl Reader>, id: ResourceId) {
    print_resource(id, riff.load::<Vec<u8>>(id).map_err(anyhow::Error::new));
}

fn inspect_riff(stream: &mut impl Reader, options: &Options) -> AResult<()> {
    let riff = Riff::new(stream)?;
    inspect_riff_contents(&riff, options)?;
    Ok(())
}

fn inspect_riff_contents(riff: &Riff<impl Reader>, options: &Options) -> AResult<()> {
    let (version, min_cast_num) = {
        let config = riff.load_num::<Config>(1024_i16.into())?;
        if !config.valid() {
            eprintln!("Configuration checksum failure!");
        }
        if options.print_config() {
            println!("{:#?}", config);
        }
        (config.version(), config.min_cast_num().into())
    };

    if options.print_casts() {
        for resource in riff.iter() {
            let id = resource.id();
            if id.os_type().as_bytes() == b"MCsL" {
                let cast_list = riff.load::<CastList>(id)?;
                println!("{:#?}", cast_list);
            }
        }
    }

    if options.list() {
        for resource in riff.iter() {
            println!("{}", resource);
        }
    }

    if options.print_file_info() {
        match riff.load_num::<FileInfo>(1024_i16.into()) {
            Ok(info) => println!("{:#?}", info),
            Err(error) => eprintln!("Error reading file info: {}", error),
        }
    }

    if let Some(resources) = options.print_resource() {
        resources.iter().for_each(|&id| {
            print_riff_resource(riff, id);
        });
    } else if options.print_resources() {
        for resource in riff.iter() {
            print_riff_resource(riff, resource.id());
        }
    } else if options.print_cast_member().is_some() || options.print_cast_members() {
        if version == ConfigVersion::Unknown {
            eprintln!("Can’t inspect cast: unknown config version!");
            return Ok(());
        }

        // TODO: Handle multiple internal casts
        match Library::from_riff(riff, 1024_i16) {
            Ok(cast) => print_cast_library(&cast, min_cast_num, options),
            Err(error) => eprintln!("Error reading cast library: {:?}", error)
        }
    }

    print_score(options, riff);

    Ok(())
}

fn inspect_riff_container(stream: impl Reader, options: &Options) -> AResult<()> {
    let riff_container = RiffContainer::new(stream)?;
    for index in 0..riff_container.len() {
        println!("\nFile {}: {}", index + 1, riff_container.filename(index).unwrap());
        if options.recursive() && riff_container.kind(index).unwrap() != ChunkFileKind::Xtra {
            match riff_container.load_file(index) {
                Ok(riff) => inspect_riff_contents(&riff, options)?,
                Err(e) => eprintln!("Could not inspect file: {}", e)
            }
        }
    }

    Ok(())
}

fn print_score(options: &Options, source: &impl ResourceSource) {
    if let Some((score_num, frames, fields)) = options.print_score() {
        let config_id = ResourceId::new(b"VWCF", score_num);
        let config = match source.load::<Config>(config_id) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("{}", e);
                return;
            }
        };

        match source.load_args::<Score>(ResourceId::new(b"VWSC", score_num), (config.version(), )) {
            Ok(score) => {
                let (start, end) = frames.unwrap_or((0, i16::MAX));
                for (i, frame) in (*score).clone().skip(start.unwrap_into()).take((end - start).unwrap_into()).enumerate() {
                    let frame_num = i16::unwrap_from(i) + start + 1;
                    match frame {
                        Ok(frame) => {
                            println!("Frame {}:", frame_num);
                            if let Some(ref fields) = fields {
                                let print_sprites = print_frame(&frame, fields);
                                if print_sprites {
                                    print_frame_sprites(&frame, fields);
                                }
                            } else {
                                println!("{:#?}", frame);
                            }
                        },
                        Err(e) => {
                            eprintln!("Error reading frame {}: {:?}", frame_num, e);
                        },
                    }
                }
            },
            Err(e) => eprintln!("{}", e),
        }
    }
}

fn read_embedded_movie(num_movies: u16, stream: impl Reader, options: &Options) -> AResult<()> {
    let rom = ResourceFile::new(stream)?;

    if options.print_config() {
        rom.iter_kind(b"VWCF").take(num_movies.into()).try_for_each(|config_id| -> AResult<()> {
            let config = rom.load::<Config>(config_id)?;
            if !config.valid() {
                eprintln!("Configuration checksum failure!");
            }
            println!("{:#?}", config);
            Ok(())
        })?;
    }

    if options.list() {
        for resource_id in rom.iter() {
            println!("{}", resource_id);
        }
    }

    if options.print_file_info() {
        // TODO: Handle multiple internal movies
        match rom.load::<FileInfo>(ResourceId::new(b"VWFI", 1024_i16)) {
            Ok(info) => println!("{:#?}", info),
            Err(error) => eprintln!("Error reading file info: {}", error),
        }
    }

    if let Some(resources) = options.print_resource() {
        resources.iter().for_each(|&id| {
            print_mac_resource(&rom, id);
        });
    } else if options.print_resources() {
        for id in rom.iter() {
            print_mac_resource(&rom, id);
        }
    } else if options.print_cast_member().is_some() || options.print_cast_members() {
        // TODO: Handle multiple internal casts
        let min_cast_num = rom.load_num::<Config>(1024_i16.into())?.min_cast_num().into();
        match Library::from_resource_source(&rom, 1024_i16) {
            Ok(cast) => print_cast_library(&cast, min_cast_num, options),
            Err(error) => eprintln!("Error reading cast library: {:?}", error)
        }
    }

    print_score(options, &rom);

    Ok(())
}

fn read_movie(info: &MovieDetectionInfo, mut stream: impl Reader, options: &Options) -> AResult<()> {
    match info.kind() {
        MovieKind::Movie | MovieKind::Cast => inspect_riff(&mut stream, options),
        MovieKind::Accelerator | MovieKind::Embedded => read_embedded_movie(1, stream, options),
    }
}

fn read_projector(
    info: &ProjectorDetectionInfo,
    mut stream: impl Reader,
    options: &Options
) -> AResult<()> {
    match info.movie() {
        MovieInfo::D3Win(movies) => {
            println!("{} embedded movies", movies.len());
            let stream = SharedStream::new(stream);
            for movie in movies {
                println!("Internal movie at {}", movie.offset());
                if options.recursive() {
                    let mut stream = stream.substream(movie.offset().into(), (movie.offset() + movie.size()).into());
                    inspect_riff(&mut stream, options)?;
                }
            }
        },
        &MovieInfo::Internal(offset) => {
            println!("Internal movie at {}", offset);
            stream.seek(SeekFrom::Start(offset.into()))?;
            inspect_riff_container(stream, options)?;
        },
        MovieInfo::External(filenames) => {
            for filename in filenames {
                println!("External movie at {}", filename);

                if options.recursive() {
                    let path = filename.to_path_lossy();
                    let mut components = path.components();
                    loop {
                        components.next();
                        let components_path = components.as_path();
                        if components_path.file_name().is_none() {
                            eprintln!("File not found");
                            break;
                        }

                        let file_path = if let Some(data_dir) = &options.data_dir {
                            let mut file_path = data_dir.clone();
                            file_path.push(components_path);
                            file_path
                        } else {
                            PathBuf::from(components_path)
                        };

                        if file_path.exists() {
                            read_file(file_path.to_str().unwrap(), options)?;
                            break;
                        }
                    }
                }
            }
        },
        MovieInfo::Embedded(num_movies) => {
            println!("{} embedded movies", num_movies);
            if options.recursive() {
                read_embedded_movie(*num_movies, stream, options)?;
            }
        },
    }
    Ok(())
}
