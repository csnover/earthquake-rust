// https://github.com/rust-lang/cargo/issues/5034
#![warn(clippy::pedantic)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::missing_errors_doc,
    clippy::non_ascii_literal,
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
    }, name, resources::{cast::{CastMap, Member, MemberId}, config::{Config, Version as ConfigVersion}}};
use libcommon::{Reader, SharedStream, encodings::MAC_ROMAN};
use libmactoolbox::{OSType, ResourceFile, ResourceId, rsid, vfs::HostFileSystem};
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
}

struct Options {
    command: Command,
    data_dir: Option<PathBuf>,
}

impl Options {
    fn detect(&self) -> bool {
        match self.command {
            Command::Detect(_) => true,
            _ => false,
        }
    }

    fn list(&self) -> bool {
        match self.command {
            Command::List(_) => true,
            _ => false,
        }
    }

    fn print_casts(&self) -> bool {
        match self.command {
            Command::PrintCasts => true,
            _ => false,
        }
    }

    fn print_cast_member(&self) -> Option<&Vec<MemberId>> {
        match self.command {
            Command::PrintCastMember(ref members) => Some(members),
            _ => None,
        }
    }

    fn print_cast_members(&self) -> bool {
        match self.command {
            Command::PrintCastMembers => true,
            _ => false,
        }
    }

    fn print_config(&self) -> bool {
        match self.command {
            Command::PrintConfig => true,
            _ => false,
        }
    }

    fn print_resource(&self) -> Option<&Vec<ResourceId>> {
        match self.command {
            Command::PrintResource(ref resources) => Some(resources),
            _ => None,
        }
    }

    fn print_resources(&self) -> bool {
        match self.command {
            Command::PrintResources => true,
            _ => false,
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
            Ok(ResourceId(os_type, resource_id))
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
            cmd => bail!("Invalid command '{}'", cmd),
        })
    } else {
        bail!("Missing command")
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
    print_resource(id, riff.load_id::<Vec<u8>>(id, &()));
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
    let config_id = if riff.has_id(rsid!(b"VWCF", 1024)) {
        Some(rsid!(b"VWCF", 1024))
    } else if riff.has_id(rsid!(b"DRCF", 1024)) {
        Some(rsid!(b"DRCF", 1024))
    } else {
        None
    };

    let (version, min_cast_num) = if let Some(config_id) = config_id {
        let config = riff.load_id::<Config>(config_id, &())?;
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
        if let Ok(cast) = riff.load_id::<CastMap>(rsid!(b"CAS*", 1024), &()) {
            for (i, &chunk_index) in cast.iter().enumerate() {
                if chunk_index > ChunkIndex::new(0) {
                    let cast_member_num = min_cast_num + (i as i16);
                    if options.print_cast_members() || options.print_cast_member().unwrap().contains(&MemberId::new(0, cast_member_num)) {
                        match riff.load::<Member>(chunk_index, &(chunk_index, version, MAC_ROMAN)) {
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

fn read_embedded_movie(num_movies: u16, stream: impl Reader, options: &Options) -> AResult<()> {
    let rom = ResourceFile::new(stream)?;

    if options.print_config() {
        for i in 0..num_movies {
            let config_id = rsid!(b"VWCF", 1024 + (i as i16) * 1000);
            if rom.contains(config_id) {
                let config = rom.load::<Config>(config_id, &())?;
                if !config.valid() {
                    eprintln!("Configuration checksum failure!");
                }
                println!("{:#?}", config);
            } else {
                eprintln!("No config chunk for movie {}!", i + 1);
            };
        }
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
