// https://github.com/rust-lang/cargo/issues/5034
#![warn(clippy::pedantic)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::map_err_ignore,
    clippy::missing_errors_doc,
    clippy::non_ascii_literal,
    clippy::option_if_let_else,
    clippy::verbose_bit_mask,
)]
#![warn(rust_2018_idioms)]

use anyhow::{bail, Context, Result as AResult};
use libearthquake::{collections::{
        riff::{ChunkIndex, Riff},
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
    }, name, player::score::Frame, player::score::Score, resources::{cast::{CastMap, Member, MemberId}, config::{Config, Version as ConfigVersion}, movie::CastList}};
use libcommon::{Reader, SharedStream, encodings::MAC_ROMAN};
use libmactoolbox::{OSType, ResourceFile, ResourceId, ResourceSource, vfs::HostFileSystem};
use pico_args::Arguments;
use std::{env, io::SeekFrom, path::{Path, PathBuf}, process::exit};

enum Command {
    Detect(bool),
    List(bool),
    PrintCasts,
    PrintCastMember(Vec<MemberId>),
    PrintCastMembers,
    PrintConfig,
    PrintResource(Vec<ResourceId>),
    PrintResources,
    PrintScore(i16, Option<(i16, i16)>, Option<Vec<String>>),
}

struct Options {
    command: Command,
    data_dir: Option<PathBuf>,
}

type PrintScoreOptions = (i16, Option<(i16, i16)>, Option<Vec<String>>);

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

fn exit_usage() -> ! {
    eprintln!(include_str!("inspect.usage"), env::args().next().unwrap_or_else(|| "inspect".to_string()));
    exit(1);
}

fn parse_fields(fields: &str) -> AResult<Vec<String>> {
    Ok(fields.split(',').map(String::from).collect::<Vec<String>>())
}

fn parse_frames(frames: &str) -> AResult<(i16, i16)> {
    match frames.split(',').take(3).collect::<Vec<_>>().as_slice() {
        [ start_frame, end_frame ] => {
            let start_frame = start_frame.parse::<i16>()
                .with_context(|| format!("Malformed start frame number '{}'", start_frame))?;
            let end_frame = if end_frame.is_empty() {
                i16::MAX
            } else {
                end_frame.parse::<i16>()
                    .with_context(|| format!("Malformed start frame number '{}'", end_frame))?
            };
            if start_frame >= end_frame {
                bail!("Start frame {} >= end frame {}", start_frame, end_frame);
            }
            Ok((start_frame - 1, end_frame - 1))
        },
        [ frame_num ] => {
            let frame_num = frame_num.parse::<i16>()
                .with_context(|| format!("Malformed frame number '{}'", frame_num))?;
            Ok((frame_num - 1, frame_num))
        },
        _ => bail!("Malformed frame range '{}'", frames)
    }
}

fn parse_member_id(id: &str) -> AResult<MemberId> {
    if let Ok(member_num) = id.parse::<i16>() {
        Ok(MemberId::new(0, member_num))
    } else {
        match id.split(',').take(3).collect::<Vec<_>>().as_slice() {
            [ lib_num, member_num ] => {
                let lib_num = lib_num.parse::<i16>()
                    .with_context(|| format!("Malformed cast library number '{}'", lib_num))?;
                let member_num = member_num.parse::<i16>()
                    .with_context(|| format!("Malformed cast member number '{}'", member_num))?;
                Ok(MemberId::new(lib_num, member_num))
            },
            _ => bail!("Malformed cast member ID '{}'", id)
        }
    }
}

fn parse_resource_id(id: &str) -> AResult<ResourceId> {
    match id.split(',').take(3).collect::<Vec<_>>().as_slice() {
        [ os_type, resource_id ] => {
            let os_type = os_type.parse::<OSType>()
                .with_context(|| format!("Malformed resource kind '{}'", os_type))?;
            let resource_id = resource_id.parse::<i16>()
                .with_context(|| format!("Malformed resource number '{}'", resource_id))?;
            Ok(ResourceId::new(os_type, resource_id))
        },
        _ => bail!("Malformed cast member ID '{}'", id)
    }
}

fn parse_command(args: &mut Arguments) -> AResult<Command> {
    if let Ok(Some(subcommand)) = args.subcommand() {
        Ok(match subcommand.as_ref() {
            "detect" => Command::Detect(args.contains("--recursive")),
            "list" => Command::List(args.contains("--recursive")),
            "print-config" => Command::PrintConfig,
            "print-cast-member" => Command::PrintCastMember(args.values_from_fn::<_, MemberId, _>("--id", parse_member_id)?),
            "print-cast-members" => Command::PrintCastMembers,
            "print-casts" => Command::PrintCasts,
            "print-resource" => Command::PrintResource(args.values_from_fn::<_, ResourceId, _>("--id", parse_resource_id)?),
            "print-resources" => Command::PrintResources,
            "print-score" => Command::PrintScore(
                args.opt_value_from_str::<_, i16>("--id")?.unwrap_or(1024),
                args.opt_value_from_fn::<_, (i16, i16), _>("--frames", parse_frames)?,
                args.opt_value_from_fn::<_, Vec<String>, _>("--fields", parse_fields)?,
            ),
            cmd => bail!("Invalid command '{}'", cmd),
        })
    } else {
        bail!("Missing command")
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
    print_resource(id, rom.load::<Vec<u8>>(id, &()));
}

fn print_resource(id: ResourceId, result: AResult<impl std::fmt::Debug>) {
    match result {
        Ok(r) => println!("{}: {:02x?}", id, r),
        Err(e) => eprintln!("Can’t inspect {}: {}", id, e)
    }
}

fn print_riff_resource(riff: &Riff<impl Reader>, id: ResourceId) {
    print_resource(id, riff.load::<Vec<u8>>(id, &()));
}

fn main() -> AResult<()> {
    eprintln!("{} file inspector", name(true));

    let mut args = Arguments::from_env();
    let command = match parse_command(&mut args) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("{}", error);
            exit_usage();
        }
    };
    let data_dir = args.opt_value_from_str::<_, PathBuf>("--data")?;
    let files = args.free()?;

    if files.is_empty() {
        eprintln!("No files specified");
        exit_usage();
    }

    let options = Options {
        command,
        data_dir,
    };

    for filename in &files {
        if files.len() > 1 {
            println!("{}:", filename);
            match read_file(&filename, &options) {
                Ok(_) => {},
                Err(e) => println!("{:?}", e),
            }
            println!();
        } else {
            read_file(&filename, &options)?;
        }
    }

    Ok(())
}

fn inspect_riff(stream: &mut impl Reader, options: &Options) -> AResult<()> {
    let riff = Riff::new(stream)?;
    inspect_riff_contents(&riff, options)?;
    Ok(())
}

fn inspect_riff_contents(riff: &Riff<impl Reader>, options: &Options) -> AResult<()> {
    let config_id = if riff.contains((b"VWCF", 1024)) {
        Some(ResourceId::new(b"VWCF", 1024))
    } else if riff.contains((b"DRCF", 1024)) {
        Some(ResourceId::new(b"DRCF", 1024))
    } else {
        None
    };

    let (version, min_cast_num) = if let Some(config_id) = config_id {
        let config = riff.load::<Config>(config_id, &())?;
        if !config.valid() {
            eprintln!("Configuration checksum failure!");
        }
        if options.print_config() {
            println!("{:#?}", config);
        }
        (config.version(), config.min_cast_num().0)
    } else {
        eprintln!("No config chunk!");
        (ConfigVersion::Unknown, 0)
    };

    if options.print_casts() {
        for resource in riff.iter() {
            let id = resource.id();
            if id.os_type().as_bytes() == b"MCsL" {
                let cast_list = riff.load::<CastList>(id, &(MAC_ROMAN, ))?;
                println!("{:?}", cast_list);
                for (i, cast) in cast_list.iter().enumerate() {
                    println!("{}: {:?}", i, cast);
                }
            }
        }
    }

    if options.list() {
        for resource in riff.iter() {
            println!("{}", resource);
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
        if let Ok(cast) = riff.load::<CastMap>(ResourceId::new(b"CAS*", 1024), &()) {
            for (i, &chunk_index) in cast.iter().enumerate() {
                if chunk_index > ChunkIndex::new(0) {
                    let cast_member_num = min_cast_num + (i as i16);
                    if options.print_cast_members() || options.print_cast_member().unwrap().contains(&MemberId::new(0, cast_member_num)) {
                        match riff.load_chunk::<Member>(chunk_index, &(chunk_index, version, MAC_ROMAN)) {
                            Ok(member) => println!("{}: {:#?}", cast_member_num, member),
                            Err(err) => println!("Failed to inspect cast member {}: {:#}", cast_member_num, err),
                        }
                    }
                }
            }
        } else {
            eprintln!("No cast library!");
        }
    }

    print_score(options, riff);

    Ok(())
}

fn inspect_riff_container(stream: impl Reader, options: &Options) -> AResult<()> {
    let riff_container = RiffContainer::new(stream)?;
    for index in 0..riff_container.len() {
        println!("\nFile {}: {}", index + 1, riff_container.filename(index).unwrap().to_string_lossy());
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
        let config = match source.load::<Config>(config_id, &()) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("{}", e);
                return;
            }
        };

        match source.load::<Score>(ResourceId::new(b"VWSC", score_num), &(config.version(), )) {
            Ok(score) => {
                let (start, end) = frames.unwrap_or((0, i16::MAX));
                for (i, frame) in (*score).clone().skip(start as usize).take((end - start) as usize).enumerate() {
                    let frame_num = i as i16 + start + 1;
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
            let config = rom.load::<Config>(config_id, &())?;
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

    if let Some(resources) = options.print_resource() {
        resources.iter().for_each(|&id| {
            print_mac_resource(&rom, id);
        });
    } else if options.print_resources() {
        for id in rom.iter() {
            print_mac_resource(&rom, id);
        }
    } else if options.print_cast_member().is_some() || options.print_cast_members() {
        todo!("D3 cast member inspection");
    }

    print_score(options, &rom);

    Ok(())
}

fn read_file(filename: &str, options: &Options) -> AResult<()> {
    let fs = HostFileSystem::new();
    let Detection { info, resource_fork, data_fork } = detect(&fs, filename)?;

    if options.detect() {
        println!("{:?}", info);
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
                println!("Internal movie at {}", movie.offset);
                if options.recursive() {
                    let mut stream = stream.substream(u64::from(movie.offset), u64::from(movie.offset + movie.size));
                    inspect_riff(&mut stream, options)?;
                }
            }
        },
        MovieInfo::Internal(offset) => {
            println!("Internal movie at {}", offset);
            stream.seek(SeekFrom::Start(u64::from(*offset)))?;
            inspect_riff_container(stream, options)?;
        },
        MovieInfo::External(filenames) => {
            for filename in filenames {
                println!("External movie at {}", filename);

                if options.recursive() {
                    let mut components = Path::new(filename).components();
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
