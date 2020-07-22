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
#![windows_subsystem = "windows"]

mod engine;
mod loader;

use anyhow::Result as AResult;
use engine::Engine;
use fluent_ergonomics::FluentErgo;
use libearthquake::detection::detect;
use libmactoolbox::{vfs::HostFileSystem, script_manager::ScriptCode};
use loader::Loader;
use num_traits::FromPrimitive;
use pico_args::Arguments;
use qt_core::{
    q_init_resource,
    QLocale,
    qs,
};
use qt_gui::QIcon;
use qt_widgets::QApplication;
use std::{env, path::PathBuf, process::exit, rc::Rc};
use strum::VariantNames;

// TODO: This imperative style of translation does not handle the case where the
// locale changes; it should register widgets so that they can be re-translated
// on the fly instead by re-setting their text.
#[macro_export]
macro_rules! tr {
    ($l: expr, $msgid: expr) => ({ $l.tr($msgid, None).unwrap_or_else(|e| e.to_string()) });
    ($l: expr, $msgid: expr, $args: tt) => ({ $l.tr($msgid, Some(&::fluent::fluent_args!$args)).unwrap_or_else(|e| e.to_string()) });
}

#[macro_export]
macro_rules! qtr {
    ($l: expr, $msgid: expr) => ({ &::qt_core::qs(&$crate::tr!($l, $msgid)) });
    ($l: expr, $msgid: expr, $args: tt) => ({ &::qt_core::qs(&$crate::tr!($l, $msgid, $args)) });
}

fn main() -> AResult<()> {
    q_init_resource!("resources");

    let mut args = Arguments::from_env();

    let mut localizer = FluentErgo::new(&unsafe {
        let qt_languages = QLocale::system().ui_languages();
        let mut languages = Vec::with_capacity(qt_languages.size() as usize);
        for lang in qt_languages.static_upcast::<qt_core::QListOfQString>().iter() {
            languages.push(lang.to_std_string().parse::<unic_langid::LanguageIdentifier>().unwrap());
        }
        languages.push("en-US".parse().unwrap());
        languages
    }[..]);
    // TODO: Add lazy-loading of other locales, maybe via q_init_resource
    localizer.add_from_text("en-US".parse().unwrap(), include_str!("../locales/en-US/main.ftl").to_owned()).unwrap();

    if args.contains("--help") {
        println!("{}", tr!(localizer, "cli_usage", [
            "exe" => env::args().next().unwrap_or_else(|| env!("CARGO_PKG_NAME").to_string())
        ]));
        for (value, &key) in ScriptCode::VARIANTS.iter().enumerate() {
            println!("    {:2}: {}", value, tr!(localizer, &format!("charset_{}", key)));
        }
        exit(0);
    }

    let charset = args.opt_value_from_str::<_, i32>("--charset")?.map(|v| ScriptCode::from_i32(v).unwrap_or(ScriptCode::Roman));
    let data_dir = args.opt_value_from_str::<_, PathBuf>("--data")?;
    let args_files = args.free()?;

    QApplication::init(|app| {
        unsafe { QApplication::set_window_icon(&QIcon::from_q_string(&qs(":/icon.png"))); }

        let files = if args_files.is_empty() {
            Loader::new(Rc::new(localizer)).exec().map_or_else(Vec::new, |file| vec![file])
        } else {
            let fs = HostFileSystem::new();
            args_files.iter().filter_map(|filename| {
                detect(&fs, &filename).map_or(None, |info| Some((filename.clone(), info.info)))
            }).collect()
        };

        if files.is_empty() {
            0
        } else {
            let mut engine = Engine::new(app, charset, data_dir, files);
            engine.exec()
        }
    })
}

include!(concat!(env!("OUT_DIR"), "/Info.plist.rs"));
