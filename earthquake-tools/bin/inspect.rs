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
    clippy::struct_excessive_bools,
    clippy::verbose_bit_mask,
)]
#![warn(rust_2018_idioms)]

use anyhow::{bail, Context, Result as AResult};
use libearthquake::{debug::{Command, MemberId, Options, read_file}, name};
use libmactoolbox::resources::{OsType, ResourceId};
use pico_args::Arguments;
use std::{env, path::PathBuf, process::exit};

fn exit_usage() -> ! {
    eprintln!(include_str!("inspect.usage"), env::args().next().unwrap_or_else(|| "inspect".to_string()));
    exit(1);
}

// https://github.com/rust-lang/rust-clippy/issues/6613
#[allow(clippy::unnecessary_wraps)]
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
        Ok(MemberId::new(0_i16, member_num))
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
            let os_type = os_type.parse::<OsType>()
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
            "print-file-info" => Command::PrintFileInfo,
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
