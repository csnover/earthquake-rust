use std::{env, fs::{File, read_dir}, io::Write};
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

fn main() {
    gen_info_plist();
    gen_resources_qrc();
    add_resources(format!("{}/{}", env::var("OUT_DIR").unwrap(), "/resources.qrc"));
    generate_cargo_keys(ConstantsFlags::SHA_SHORT
        | ConstantsFlags::SEMVER
        | ConstantsFlags::COMMIT_DATE
        | ConstantsFlags::REBUILD_ON_HEAD_CHANGE).unwrap();
}
