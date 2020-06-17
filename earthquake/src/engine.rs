use cpp_core::Ptr;
use libmactoolbox::script_manager::ScriptCode;
use std::path::PathBuf;
use qt_widgets::QApplication;

pub(crate) struct Engine {
    app: Ptr<QApplication>,
    charset: Option<ScriptCode>,
    data_dir: Option<PathBuf>,
    files: Vec<String>,
}

impl Engine {
    pub fn new(app: Ptr<QApplication>, charset: Option<ScriptCode>, data_dir: Option<PathBuf>, files: Vec<String>) -> Self {
        Self {
            app,
            charset,
            data_dir,
            files,
        }
    }

    pub fn exec(&mut self) -> i32 {
        unsafe { QApplication::exec() }
    }
}
