use cpp_build::Config as CppConfig;
use std::{env, fs::{File, read_dir}, io::Write, process::Command};
use qt_ritual_build::add_resources;
use vergen::{ConstantsFlags, generate_cargo_keys};

fn escape_xml(str: &str) -> String {
    str.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn make_file(filename: &str) -> std::io::Result<File> {
    File::create(format!("{}/{}", env::var("OUT_DIR").unwrap(), filename))
}

fn gen_resources_qrc() {
    let mut file = make_file("resources.qrc").expect("Could not create resources.qrc");
    write!(&mut file, r#"<!DOCTYPE RCC><RCC version="1.0"><qresource>"#).unwrap();
    for resource in read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/")).unwrap() {
        let resource = resource.unwrap();
        if resource.metadata().unwrap().is_file() {
            let filename = resource.file_name().to_str().unwrap().to_owned();
            if !filename.starts_with('.') {
                let dir = escape_xml(env!("CARGO_MANIFEST_DIR"));
                let filename = escape_xml(&filename);
                write!(&mut file, r#"<file alias="{1}">{0}/resources/{1}</file>"#, dir, filename).unwrap();
            }
        }
    }
    write!(&mut file, "</qresource></RCC>").unwrap();
}

fn gen_info_plist() {
    let mut file = make_file("Info.plist.rs").expect("Could not create Info.plist");
    // TODO: Get version, year, and bundle_id out of vergen/Git
    let plist = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
        <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
        <plist version="1.0">
        <dict>
            <key>CFBundleExecutable</key>
            <string>{exe}</string>
            <key>CFBundleGetInfoString</key>
            <string>{version} © {author}</string>
            <key>CFBundleIconFile</key>
            <string>{exe}.icns</string>
            <key>CFBundleIdentifier</key>
            <string>{bundle_id}</string>
            <key>CFBundleInfoDictionaryVersion</key>
            <string>6.0</string>
            <key>CFBundleName</key>
            <string>{name}</string>
            <key>CFBundlePackageType</key>
            <string>APPL</string>
            <key>CFBundleShortVersionString</key>
            <string>{version}</string>
            <key>NSHighResolutionCapable</key>
            <true/>
            <key>NSPrincipalClass</key>
            <string>NSApplication</string>
            <key>NSQuitAlwaysKeepsWindows</key>
            <false/>
        </dict>
        </plist>"#,
        author = env!("CARGO_PKG_AUTHORS"),
        bundle_id = "com.github.earthquake-project.earthquake-rust",
        exe = env!("CARGO_PKG_VERSION"),
        name = "Earthquake",
        version = env!("CARGO_PKG_VERSION"),
    );

    write!(&mut file, r#"
            #[cfg(target_os = "macos")]
            #[link_section = "__TEXT,__info_plist"]
            #[used]
            static INFO_PLIST: [u8; {size}] = {plist:?};
            "#,
        size = plist.as_bytes().len(),
        plist = plist.as_bytes()).unwrap();
}

// Taken from qmetaobject-rs
fn qmake_query(var: &str) -> String {
    let qmake = std::env::var("QMAKE").unwrap_or_else(|_| "qmake".to_string());
    String::from_utf8(
        Command::new(qmake)
            .env("QT_SELECT", "qt5")
            .args(&["-query", var])
            .output()
            .expect("Failed to execute qmake. Make sure 'qmake' is in your path")
            .stdout,
    )
    .expect("UTF-8 conversion failed")
}

// Taken from qmetaobject-rs
fn gen_qt_cargo_keys() {
    let qt_include_path = qmake_query("QT_INSTALL_HEADERS").trim().to_owned();
    let qt_library_path = qmake_query("QT_INSTALL_LIBS").trim().to_owned();
    let mut config = CppConfig::new();

    if cfg!(target_os = "macos") {
        config.flag("-F").flag(&qt_library_path);
    }

    config.include(&qt_include_path).build("src/main.rs");

    let macos_lib_search = if cfg!(target_os = "macos") { "=framework" } else { "" };
    let macos_lib_framework = if cfg!(target_os = "macos") { "" } else { "5" };

    // https://github.com/mystor/rust-cpp/issues/84
    for source_file in read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/src/qt/")).unwrap() {
        let path = source_file.unwrap().path().to_owned();
        let path = path.strip_prefix(env!("CARGO_MANIFEST_DIR")).unwrap();
        if path.file_name().unwrap() != "mod.rs" {
            println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
        }
    }

    println!("cargo:rustc-link-search{}={}", macos_lib_search, qt_library_path);
    println!("cargo:rustc-link-lib{}=Qt{}Widgets", macos_lib_search, macos_lib_framework);
    println!("cargo:rustc-link-lib{}=Qt{}Gui", macos_lib_search, macos_lib_framework);
    println!("cargo:rustc-link-lib{}=Qt{}Core", macos_lib_search, macos_lib_framework);
}

fn main() {
    gen_info_plist();
    gen_resources_qrc();
    add_resources(format!("{}/{}", env::var("OUT_DIR").unwrap(), "/resources.qrc"));
    generate_cargo_keys(ConstantsFlags::SHA_SHORT
        | ConstantsFlags::SEMVER
        | ConstantsFlags::COMMIT_DATE
        | ConstantsFlags::REBUILD_ON_HEAD_CHANGE).unwrap();
    gen_qt_cargo_keys();
}
