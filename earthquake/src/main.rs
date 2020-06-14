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

use anyhow::Result as AResult;
use cpp_core::{NullPtr, Ptr, StaticUpcast};
use libearthquake::{
    detection::{detect, FileType, movie::Kind as MovieKind},
    name,
    version,
};
use libmactoolbox::script_manager::ScriptCode;
use pico_args::Arguments;
use qt_core::{
    q_dir::Filter as DirFilter,
    q_init_resource,
    AlignmentFlag,
    QBox,
    QObject,
    QPtr,
    qs,
    QVariant,
    slot,
    SlotNoArgs,
    TextFormat,
    TextInteractionFlag,
    WidgetAttribute,
    WindowType,
};
use qt_gui::{
    QFont,
    QIcon,
    QPainter,
    QPixmap,
};
use qt_widgets::{
    q_action::MenuRole,
    q_combo_box::SizeAdjustPolicy,
    q_completer::CompletionMode,
    q_dialog::DialogCode,
    q_dialog_button_box::ButtonRole,
    q_layout::SizeConstraint,
    q_message_box::{Icon as MBIcon, StandardButton as MBButton},
    QAction,
    QApplication,
    QBoxLayout,
    QComboBox,
    QCompleter,
    QDialog,
    QDialogButtonBox,
    QFileDialog,
    QFileSystemModel,
    QFormLayout,
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QMenuBar,
    QMessageBox,
    QPushButton,
    QStackedLayout,
    QTabWidget,
    QVBoxLayout,
    QWidget,
};
use std::{cell::RefCell, env, path::{Path, PathBuf}, process::exit, rc::Rc};
use strum::VariantNames;

struct FileWidget {
    layout: QBox<QHBoxLayout>,
    input: QBox<QLineEdit>,
    browse: QBox<QPushButton>,
    // This is held only by weak reference by the QLineEdit so it
    // must be retained or else auto-completion will not work
    _completer: QBox<QCompleter>,
}

impl FileWidget {
    pub fn new(for_directory: bool) -> Self {
        unsafe {
            let layout = QHBoxLayout::new_0a();

            let input = QLineEdit::new();
            let completer = QCompleter::new();
            let model = QFileSystemModel::new_1a(&completer);
            model.set_root_path(&qs(""));
            if for_directory {
                model.set_filter(DirFilter::Dirs | DirFilter::NoDotAndDotDot);
            }
            completer.set_model(&model);
            completer.set_completion_mode(CompletionMode::PopupCompletion);
            input.set_completer(&completer);

            let file_name = if for_directory { "data" } else { "movie.dxr" };

            input.set_placeholder_text(&qs(if cfg!(windows) {
                format!(r"C:\path\to\{}", file_name)
            } else {
                format!("/path/to/{}", file_name)
            }));

            layout.add_widget_2a(&input, 1);
            layout.set_spacing(4);

            let browse = QPushButton::from_q_string(&qs(if for_directory { "B&rowse" } else { "&Browse" }));
            layout.add_widget(&browse);

            Self {
                layout,
                input,
                browse,
                _completer: completer,
            }
        }
    }
}

struct InfoWidget {
    stack: QBox<QStackedLayout>,
    not_loaded_index: i32,
    loaded_index: i32,
    file_name: QBox<QLabel>,
    kind: QBox<QLabel>,
}

impl InfoWidget {
    fn new(parent: &QTabWidget) -> InfoWidget {
        unsafe {
            let tab = QWidget::new_0a();
            let stack = QStackedLayout::new();
            tab.set_layout(&stack);

            let not_loaded_index = stack.add_widget(&{
                let not_loaded = QLabel::from_q_string(&qs("No file loaded"));
                not_loaded.set_contents_margins_4a(0, 0, 0, 14);
                not_loaded.set_alignment(AlignmentFlag::AlignCenter.into());
                not_loaded
            });

            let loaded = QWidget::new_0a();
            loaded.set_contents_margins_4a(0, 0, 0, 14);
            let layout = QVBoxLayout::new_1a(&loaded);
            layout.add_stretch_1a(1);

            let file_name = QLabel::new();
            file_name.set_text_format(TextFormat::PlainText);
            file_name.set_font(&{
                let font = QFont::new();
                font.set_bold(true);
                font
            });
            layout.add_widget(&file_name);

            let kind = QLabel::new();
            kind.set_text_format(TextFormat::PlainText);
            layout.add_widget(&kind);

            layout.add_stretch_1a(1);

            let loaded_index = stack.add_widget(&loaded);
            stack.set_current_index(not_loaded_index);

            parent.add_tab_2a(&tab, &qs("File &info"));

            InfoWidget {
                stack,
                not_loaded_index,
                loaded_index,
                file_name,
                kind,
            }
        }
    }
}

struct OptionsWidget {
    tab_index: i32,
    charset: QBox<QComboBox>,
    data_dir: FileWidget,
}

impl OptionsWidget {
    fn new(parent: &QTabWidget) -> OptionsWidget {
        unsafe {
            let tab = QWidget::new_0a();
            let layout = QFormLayout::new_0a();
            tab.set_layout(&layout);
            let tab_index = parent.add_tab_2a(&tab, &qs("&Options"));
            parent.set_tab_enabled(tab_index, false);

            let charset = Self::build_charset_box(&layout);
            let data_dir = Self::build_data_box(&layout);

            OptionsWidget {
                tab_index,
                charset,
                data_dir,
            }
        }
    }

    unsafe fn build_charset_box(parent: &QFormLayout) -> QBox<QComboBox> {
        let charset = QComboBox::new_0a();
        charset.set_size_adjust_policy(SizeAdjustPolicy::AdjustToContentsOnFirstShow);
        for (value, &key) in ScriptCode::VARIANTS.iter().enumerate() {
            charset.add_item_q_string_q_variant(&qs(key), &QVariant::from_int(value as i32));
        }
        parent.add_row_q_string_q_widget(&qs("&Character set:"), &charset);
        charset
    }

    unsafe fn build_data_box(parent: &QFormLayout) -> FileWidget {
        let file_widget = FileWidget::new(true);
        let label = QLabel::from_q_string(&qs("&Data directory:"));
        parent.add_row_q_widget_q_layout(&label, &file_widget.layout);
        label.set_buddy(&file_widget.input);
        file_widget
    }
}

struct TabsWidget {
    tabs: QBox<QTabWidget>,
    info: InfoWidget,
    options: OptionsWidget,
}

impl TabsWidget {
    fn new(parent: &QBoxLayout) -> TabsWidget {
        unsafe {
            let tabs = QTabWidget::new_0a();
            let info = InfoWidget::new(&tabs);
            let options = OptionsWidget::new(&tabs);
            parent.add_widget(&tabs);

            TabsWidget {
                tabs,
                info,
                options,
            }
        }
    }
}

struct Loader {
    about_box: RefCell<QBox<QDialog>>,
    about_action: QPtr<QAction>,
    about_license_action: QPtr<QAction>,
    dialog: QBox<QDialog>,
    file: FileWidget,
    filename: RefCell<Option<String>>,
    tabs: TabsWidget,
    ok_button: QPtr<QPushButton>,
    cancel_button: QPtr<QPushButton>,
}

impl StaticUpcast<QObject> for Loader {
    unsafe fn static_upcast(ptr: Ptr<Self>) -> Ptr<QObject> {
        ptr.dialog.as_ptr().static_upcast()
    }
}

impl Loader {
    fn new() -> Rc<Self> {
        unsafe {
            let (dialog, dialog_layout) = Self::build_window();
            let (about_action, about_license_action) = Self::build_menu(&dialog);
            let file = Self::build_file_box(&dialog_layout);
            let tabs = TabsWidget::new(&dialog_layout);

            let buttons = QDialogButtonBox::new();
            let ok_button = buttons.add_button_q_string_button_role(&qs("&Play"), ButtonRole::AcceptRole);
            ok_button.set_disabled(true);
            let cancel_button = buttons.add_button_q_string_button_role(&qs("&Quit"), ButtonRole::RejectRole);
            dialog_layout.add_widget(&buttons);

            let this = Rc::new(Self {
                about_box: RefCell::new(QBox::null()),
                about_action,
                about_license_action,
                cancel_button,
                dialog,
                file,
                filename: RefCell::new(None),
                ok_button,
                tabs,
            });
            this.init();
            this
        }
    }

    unsafe fn about_logo() -> cpp_core::CppBox<QPixmap> {
        let logo = QPixmap::from_q_string(&qs(":/logo.png"));
        logo.set_device_pixel_ratio(2.0);

        let paint = QPainter::new_1a(&logo);
        let mut x = 383;
        let y = 197;
        for (index, version) in env!("CARGO_PKG_VERSION").split('.').take(2).enumerate() {
            if index != 0 {
                if version == "0" {
                    break;
                }
                let dot = QPixmap::from_q_string(&qs(":/logo-dot.png"));
                dot.set_device_pixel_ratio(2.0);
                paint.draw_pixmap_2_int_q_pixmap(x, y, &dot);
                x += dot.width() / dot.device_pixel_ratio() as i32;
            }
            let digit = QPixmap::from_q_string(&qs(format!(":/logo-{}.png", version)));
            digit.set_device_pixel_ratio(2.0);
            paint.draw_pixmap_2_int_q_pixmap(x, y, &digit);
            x += digit.width() / digit.device_pixel_ratio() as i32;
        }

        logo
    }

    unsafe fn about_text() -> String {
        let copyright_year = option_env!("VERGEN_COMMIT_DATE").map_or_else(String::new, |date| {
            date.split('-').next().unwrap().to_string() + " "
        });

        let actions = {
            let mut actions = String::new();
            let has_homepage = option_env!("CARGO_PKG_HOMEPAGE").is_some();
            if has_homepage || option_env!("CARGO_PKG_REPOSITORY").is_some() {
                actions += r#"<hr><div>"#;
                if let Some(homepage) = option_env!("CARGO_PKG_HOMEPAGE") {
                    actions += &format!(r#"<a style="color: black" href="{}">Home page</a>"#, homepage);
                }
                if let Some(repository) = option_env!("CARGO_PKG_REPOSITORY") {
                    if has_homepage {
                        actions += " &nbsp;·&nbsp; ";
                    }
                    actions += &format!(r#"<a style="color: black" href="{0}">Repository</a> &nbsp;·&nbsp;
                        <a style="color: black" href="{0}/issues/new">Report a bug</a>"#, repository);
                }
                actions += r"</div>";
            }
            actions
        };

        format!("<div>© {}{}</div>{}",
            copyright_year,
            env!("CARGO_PKG_AUTHORS"),
            actions,
        )
    }

    unsafe fn build_file_box(parent: &QBoxLayout) -> FileWidget {
        let layout = QVBoxLayout::new_0a();
        layout.set_spacing(2);

        let label = QLabel::from_q_string(&qs("Movie or projector &file:"));
        layout.add_widget(&label);
        let file_widget = FileWidget::new(false);
        layout.add_layout_1a(&file_widget.layout);
        label.set_buddy(&file_widget.input);
        parent.add_layout_1a(&layout);
        file_widget
    }

    unsafe fn build_menu(parent: &QBox<QDialog>) -> (QPtr<QAction>, QPtr<QAction>) {
        let menu_bar = QMenuBar::new_1a(parent);
        menu_bar.set_native_menu_bar(true);
        let menu = menu_bar.add_menu_q_string(&qs("&Help"));
        let about_action = menu.add_action_q_string(&qs(format!("&About {}", name(false))));
        let about_license_action = menu.add_action_q_string(&qs("About &License"));
        about_license_action.set_menu_role(MenuRole::AboutQtRole);
        (about_action, about_license_action)
    }

    unsafe fn build_window() -> (QBox<QDialog>, QBox<QVBoxLayout>) {
        let dialog = QDialog::new_0a();
        dialog.set_window_title(&qs(name(true)));
        dialog.set_modal(true);

        let dialog_layout = QVBoxLayout::new_1a(&dialog);
        dialog_layout.set_size_constraint(SizeConstraint::SetFixedSize);

        (dialog, dialog_layout)
    }

    fn detection_failure(&self, message: &str, detailed_text: Option<&str>) {
        unsafe {
            let message_box = QMessageBox::from_icon2_q_string_q_flags_standard_button_q_widget(
                MBIcon::Warning,
                &qs("Detection failed"),
                &qs(message),
                MBButton::Ok.into(),
                &self.dialog,
            );

            if let Some(detailed_text) = detailed_text {
                message_box.set_detailed_text(&qs(detailed_text));
            }

            if let Some(url) = option_env!("CARGO_PKG_REPOSITORY") {
                let url = format!("{}/issues/new", url);
                message_box.set_informative_text(&qs(format!("If you think this file is a valid Director movie or projector, please <a href=\"{}\">send a sample</a>.", url)));
            }

            message_box.exec();
        }
    }

    fn exec(&self) -> Option<String> {
        unsafe {
            if self.dialog.exec() == DialogCode::Accepted.to_int() {
                self.filename.borrow().clone()
            } else {
                None
            }
        }
    }

    unsafe fn init(self: &Rc<Self>) {
        self.file.input
            .return_pressed()
            .connect(&self.slot_on_input());
        self.file.browse
            .clicked()
            .connect(&self.slot_on_browse());
        self.tabs.options.data_dir.browse
            .clicked()
            .connect(&self.slot_on_data_dir_browse());
        self.ok_button
            .clicked()
            .connect(&self.slot_on_accept());
        self.cancel_button
            .clicked()
            .connect(&self.slot_on_cancel());
        self.about_action
            .triggered()
            .connect(&self.slot_on_about());
        self.about_license_action
            .triggered()
            .connect(&self.slot_on_about_license());
    }

    #[slot(SlotNoArgs)]
    unsafe fn on_about(self: &Rc<Self>) {
        if let Ok(message_box) = self.about_box.try_borrow() {
            if !message_box.is_null() {
                message_box.show();
                message_box.raise();
                message_box.activate_window();
                return;
            }
        }

        let message_box = QDialog::new_2a(
            &self.dialog,
            WindowType::MSWindowsFixedSizeDialogHint
            | WindowType::WindowTitleHint
            | WindowType::WindowSystemMenuHint
            | WindowType::WindowCloseButtonHint
        );

        message_box.set_modal(true);
        message_box.set_attribute_1a(WidgetAttribute::WADeleteOnClose);

        if cfg!(target_os = "macos") {
        } else {
            message_box.set_window_title(&qs(format!("About {}", name(false))));
        }

        message_box.set_style_sheet(&qs("* { color: black; background: white; }"));

        let layout = QVBoxLayout::new_0a();
        layout.set_size_constraint(SizeConstraint::SetFixedSize);
        layout.set_contents_margins_4a(0, 0, 0, 10);
        message_box.set_layout(&layout);

        layout.add_widget(&{
            let about_label = QLabel::new();
            about_label.set_pixmap(&Self::about_logo());
            about_label.set_contents_margins_1a(&{
                let margins = about_label.contents_margins();
                margins.set_bottom(2);
                margins
            });
            about_label
        });

        layout.add_widget(&{
            let version_label = QLabel::from_q_string(&qs(version()));
            version_label.set_text_format(TextFormat::PlainText);
            version_label.set_text_interaction_flags(TextInteractionFlag::TextBrowserInteraction.into());
            version_label.set_alignment(AlignmentFlag::AlignHCenter.into());
            version_label
        });

        layout.add_widget(&{
            let about_text_label = QLabel::from_q_string(&qs(Self::about_text()));
            about_text_label.set_alignment(AlignmentFlag::AlignHCenter.into());
            about_text_label
        });

        if cfg!(target_os = "macos") {
            message_box.show();
            message_box.raise();
            message_box.activate_window();
            self.about_box.replace(message_box);
        } else {
            message_box.exec();
        }
    }

    #[slot(SlotNoArgs)]
    unsafe fn on_about_license(self: &Rc<Self>) {
        let copyright_year = option_env!("VERGEN_COMMIT_DATE").map_or_else(String::new, |date| {
            date.split('-').next().unwrap().to_string() + " "
        });

        let license = format!(r#"<b>Copyright {}{}</b><br>
        <br>
        <span style="font-weight: normal">
        Licensed under the Apache License, Version 2.0 (the "License");
        you may not use this file except in compliance with the License.
        You may obtain a copy of the License at<br>
        <br>
        <a href="https://www.apache.org/licenses/LICENSE-2.0">https://www.apache.org/licenses/LICENSE-2.0</a><br>
        <br>
        Unless required by applicable law or agreed to in writing, software
        distributed under the License is distributed on an "AS IS" BASIS,
        WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
        See the License for the specific language governing permissions and
        limitations under the License.
        </span>"#, copyright_year, "Earthquake Project contributors");

        QMessageBox::about(&self.dialog, &qs(name(false)), &qs(license));
    }

    #[slot(SlotNoArgs)]
    unsafe fn on_browse(self: &Rc<Self>) {
        let path_str = QFileDialog::get_open_file_name_6a(
            &self.dialog,
            &qs("Find projector or movie"),
            &self.file.input.text(),
            &qs("Projectors (*.exe *.app *.rsrc *.bin);;Movies (*.dir *.dxr *.mmm);;All files (*)"),
            NullPtr,
            0.into());
        if !path_str.is_empty() {
            self.validate_input(path_str.to_std_string());
        }
    }

    #[slot(SlotNoArgs)]
    unsafe fn on_accept(self: &Rc<Self>) {
        self.dialog.accept()
    }

    #[slot(SlotNoArgs)]
    unsafe fn on_cancel(self: &Rc<Self>) {
        self.dialog.reject();
    }

    #[slot(SlotNoArgs)]
    unsafe fn on_data_dir_browse(self: &Rc<Self>) {
        let path_str = QFileDialog::get_existing_directory_3a(
            &self.dialog,
            &qs("Find data directory"),
            &self.tabs.options.data_dir.input.text(),
        );
        if !path_str.is_empty() {
            self.tabs.options.data_dir.input.set_text(&path_str);
        }
    }

    #[slot(SlotNoArgs)]
    unsafe fn on_input(self: &Rc<Self>) {
        self.validate_input(self.file.input.text().to_std_string());
    }

    unsafe fn validate_input(self: &Rc<Self>, chosen_path: String) {
        let is_valid = if chosen_path.is_empty() {
            false
        } else {
            match detect(&chosen_path) {
                Ok(info) => {
                    match info {
                        FileType::Projector(info, ..) => {
                            self.tabs.info.file_name.set_text(&qs(info.name().unwrap_or(&String::from("Unknown"))));
                            self.tabs.info.kind.set_text(&qs(format!("Director {} for {} projector", info.version(), info.config().platform())));
                            // TODO: Heuristic detection of character set
                            self.tabs.options.charset.set_current_index(0);
                            true
                        },
                        FileType::Movie(info, ..) if info.kind() != MovieKind::Cast => {
                            let path = Path::new(&chosen_path);
                            self.tabs.info.file_name.set_text(&qs(path.file_stem().unwrap().to_string_lossy()));
                            self.tabs.info.kind.set_text(&qs(format!("Director {} {}", info.version(), info.kind())));
                            // TODO: Heuristic detection of character set
                            self.tabs.options.charset.set_current_index(0);
                            true
                        },
                        _ => {
                            self.detection_failure("Cannot play cast libraries.", None);
                            false
                        }
                    }
                },
                Err(e) => {
                    let mut reasons = String::new();
                    for reason in e.chain().skip(1) {
                        if !reasons.is_empty() {
                            reasons += "\n";
                        }
                        reasons += &format!("• {}", reason);
                    }

                    self.detection_failure(&e.to_string(), Some(&reasons));
                    false
                },
            }
        };

        if is_valid {
            self.file.input.set_text(&qs(&chosen_path));
            *self.filename.borrow_mut() = Some(chosen_path);
        } else {
            *self.filename.borrow_mut() = None;
        }

        self.tabs.tabs.set_tab_enabled(self.tabs.options.tab_index, is_valid);
        self.tabs.info.stack.set_current_index(if is_valid { self.tabs.info.loaded_index } else { self.tabs.info.not_loaded_index });
        self.ok_button.set_disabled(!is_valid);
    }
}

fn main() -> AResult<()> {
    let mut args = Arguments::from_env();

    if args.contains("--help") {
        print!(include_str!("main.usage"), env::args().next().unwrap_or_else(|| env!("CARGO_PKG_NAME").to_string()));
        for (value, &key) in ScriptCode::VARIANTS.iter().enumerate() {
            println!("    {:2}: {}", value, key);
        }
        exit(1);
    }

    let _data_dir = args.opt_value_from_str::<_, PathBuf>("--data")?;
    let _charset = args.opt_value_from_str::<_, i32>("--charset")?;
    let files = args.free()?;

    QApplication::init(|_| unsafe {
        q_init_resource!("resources");
        QApplication::set_window_icon(&QIcon::from_q_string(&qs(":/icon.png")));
        let filename = if files.is_empty() {
            let ask = Loader::new();
            ask.exec()
        } else {
            Some(files[0].clone())
        };

        if filename.is_some() {
            QApplication::exec()
        } else {
            0
        }
    })
}

include!(concat!(env!("OUT_DIR"), "/Info.plist.rs"));
