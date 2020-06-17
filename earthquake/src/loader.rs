
use cpp_core::{CppBox, NullPtr, Ptr, StaticUpcast};
use crate::{qtr, tr};
use fluent_ergonomics::FluentErgo;
use libearthquake::{
    detection::{detect, FileType, movie::Kind as MovieKind},
    name,
    version,
};
use libmactoolbox::script_manager::ScriptCode;
use qt_core::{
    q_dir::Filter as DirFilter,
    AlignmentFlag,
    QBox,
    QObject,
    QPtr,
    qs,
    QString,
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
    QPainter,
    QPixmap,
};
use qt_widgets::{
    q_action::MenuRole,
    q_combo_box::SizeAdjustPolicy,
    q_completer::CompletionMode,
    q_dialog::DialogCode,
    q_dialog_button_box::ButtonRole,
    q_file_dialog::Option as FileDialogOption,
    q_layout::SizeConstraint,
    q_message_box::{Icon as MBIcon, StandardButton as MBButton},
    QAction,
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
use std::{cell::RefCell, env, path::Path, rc::Rc};
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
    pub fn new(l: &FluentErgo, for_directory: bool) -> Self {
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

            let key = if for_directory { "data-dir" } else { "file-load" };

            input.set_placeholder_text(qtr!(l, &format!("{}_placeholder", key), [
                "os" => env::consts::OS
            ]));

            layout.add_widget_2a(&input, 1);
            layout.set_spacing(4);

            let browse = QPushButton::from_q_string(qtr!(l, &format!("{}_browse-action", key)));
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
    fn new(l: &FluentErgo, parent: &QTabWidget) -> InfoWidget {
        unsafe {
            let tab = QWidget::new_0a();
            let stack = QStackedLayout::new();
            tab.set_layout(&stack);

            let not_loaded_index = stack.add_widget(&{
                let not_loaded = QLabel::from_q_string(qtr!(l, "no-file-loaded"));
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

            parent.add_tab_2a(&tab, qtr!(l, "tabs_file-info"));

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
    fn new(l: &FluentErgo, parent: &QTabWidget) -> OptionsWidget {
        unsafe {
            let tab = QWidget::new_0a();
            let layout = QFormLayout::new_0a();
            tab.set_layout(&layout);
            let tab_index = parent.add_tab_2a(&tab, qtr!(l, "tabs_options"));
            parent.set_tab_enabled(tab_index, false);

            let charset = Self::build_charset_box(l, &layout);
            let data_dir = Self::build_data_box(l, &layout);

            OptionsWidget {
                tab_index,
                charset,
                data_dir,
            }
        }
    }

    unsafe fn build_charset_box(l: &FluentErgo, parent: &QFormLayout) -> QBox<QComboBox> {
        let charset = QComboBox::new_0a();
        charset.set_size_adjust_policy(SizeAdjustPolicy::AdjustToContentsOnFirstShow);
        for (value, &key) in ScriptCode::VARIANTS.iter().enumerate() {
            charset.add_item_q_string_q_variant(qtr!(l, &format!("charset_{}", key)), &QVariant::from_int(value as i32));
        }
        parent.add_row_q_string_q_widget(qtr!(l, "charset_label"), &charset);
        charset
    }

    unsafe fn build_data_box(l: &FluentErgo, parent: &QFormLayout) -> FileWidget {
        let file_widget = FileWidget::new(l, true);
        let label = QLabel::from_q_string(qtr!(l, "data-dir_label"));
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
    fn new(l: &FluentErgo, parent: &QBoxLayout) -> TabsWidget {
        unsafe {
            let tabs = QTabWidget::new_0a();
            let info = InfoWidget::new(l, &tabs);
            let options = OptionsWidget::new(l, &tabs);
            parent.add_widget(&tabs);

            TabsWidget {
                tabs,
                info,
                options,
            }
        }
    }
}

pub(crate) struct Loader {
    about_box: RefCell<QBox<QDialog>>,
    about_action: QPtr<QAction>,
    about_license_action: QPtr<QAction>,
    dialog: QBox<QDialog>,
    file: FileWidget,
    filename: RefCell<Option<String>>,
    l: Rc<FluentErgo>,
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
    pub fn new(l: Rc<FluentErgo>) -> Rc<Self> {
        unsafe {
            let (dialog, dialog_layout) = Self::build_window();
            let (about_action, about_license_action) = Self::build_menu(l.as_ref(), &dialog);
            let file = Self::build_file_box(l.as_ref(), &dialog_layout);
            let tabs = TabsWidget::new(l.as_ref(), &dialog_layout);

            let buttons = QDialogButtonBox::new();
            let ok_button = buttons.add_button_q_string_button_role(qtr!(l, "play-action"), ButtonRole::AcceptRole);
            ok_button.set_disabled(true);
            let cancel_button = buttons.add_button_q_string_button_role(qtr!(l, "quit-action"), ButtonRole::RejectRole);
            dialog_layout.add_widget(&buttons);

            let loader = Rc::new(Self {
                about_box: RefCell::new(QBox::null()),
                about_action,
                about_license_action,
                cancel_button,
                dialog,
                file,
                filename: RefCell::new(None),
                l,
                ok_button,
                tabs,
            });
            loader.init();
            loader
        }
    }

    pub fn exec(&self) -> Option<String> {
        unsafe {
            if self.dialog.exec() == DialogCode::Accepted.to_int() {
                self.filename.borrow().clone()
            } else {
                None
            }
        }
    }

    unsafe fn about_logo() -> CppBox<QPixmap> {
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

    unsafe fn about_text(l: &FluentErgo) -> String {
        let copyright_year = option_env!("VERGEN_COMMIT_DATE").map_or_else(String::new, |date| {
            date.split('-').next().unwrap().to_string() + " "
        });

        let actions = {
            let mut actions = String::new();
            let has_homepage = option_env!("CARGO_PKG_HOMEPAGE").is_some();
            if has_homepage || option_env!("CARGO_PKG_REPOSITORY").is_some() {
                actions += r#"<hr><div>"#;
                if let Some(homepage) = option_env!("CARGO_PKG_HOMEPAGE") {
                    actions += &format!(
                        r#"<a style="color: black" href="{}">{}</a>"#,
                        homepage,
                        tr!(l, "about_home-page-link"),
                    );
                }
                if let Some(repository) = option_env!("CARGO_PKG_REPOSITORY") {
                    if has_homepage {
                        actions += " &nbsp;·&nbsp; ";
                    }
                    actions += &format!(r#"<a style="color: black" href="{0}">{1}</a> &nbsp;·&nbsp;
                        <a style="color: black" href="{0}/issues/new">{2}</a>"#,
                        repository,
                        tr!(l, "about_repository-link"),
                        tr!(l, "about_report-bug-link"),
                    );
                }
                actions += r"</div>";
            }
            actions
        };

        format!("<div>{}</div>{}",
            tr!(l, "about_copyright", [
                "year" => copyright_year,
                "author" => env!("CARGO_PKG_AUTHORS")
            ]),
            actions,
        )
    }

    unsafe fn build_file_box(l: &FluentErgo, parent: &QBoxLayout) -> FileWidget {
        let layout = QVBoxLayout::new_0a();
        layout.set_spacing(2);

        let label = QLabel::from_q_string(qtr!(l, "file-load_label"));
        layout.add_widget(&label);
        let file_widget = FileWidget::new(l, false);
        layout.add_layout_1a(&file_widget.layout);
        label.set_buddy(&file_widget.input);
        parent.add_layout_1a(&layout);
        file_widget
    }

    unsafe fn build_menu(l: &FluentErgo, parent: &QBox<QDialog>) -> (QPtr<QAction>, QPtr<QAction>) {
        let menu_bar = QMenuBar::new_1a(parent);
        menu_bar.set_native_menu_bar(true);
        let menu = menu_bar.add_menu_q_string(qtr!(l, "help-menu"));
        let about_action = menu.add_action_q_string(qtr!(l, "help-menu_about", [ "app_name" => name(false) ]));
        let about_license_action = menu.add_action_q_string(qtr!(l, "help-menu_about-license"));
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
                qtr!(self.l, "detection-failed_error"),
                &qs(message),
                MBButton::Ok.into(),
                &self.dialog,
            );

            if let Some(detailed_text) = detailed_text {
                message_box.set_detailed_text(&qs(detailed_text));
            }

            if let Some(url) = option_env!("CARGO_PKG_REPOSITORY") {
                message_box.set_informative_text(qtr!(
                    self.l,
                    "detection-failed_message-html",
                    [ "url" => format!("{}/issues/new", url) ]
                ));
            }

            message_box.exec();
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
            message_box.set_window_title(qtr!(self.l, "about_window-title", [ "app_name" => name(false) ]));
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
            let about_text_label = QLabel::from_q_string(&qs(Self::about_text(self.l.as_ref())));
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

        let license = format!(r#"<b>{}</b><br><br><span style="font-weight: normal">{}</span>"#,
            tr!(self.l, "license_copyright", [
                "maybe_year" => copyright_year,
                "author" => env!("CARGO_PKG_AUTHORS")
            ]),
            tr!(self.l, "license_license-html")
        );

        QMessageBox::about(&self.dialog, &qs(name(false)), &qs(license));
    }

    #[slot(SlotNoArgs)]
    unsafe fn on_browse(self: &Rc<Self>) {
        let path_str = QFileDialog::get_open_file_name_6a(
            &self.dialog,
            qtr!(self.l, "file-load_browse-title"),
            &self.file.input.text(),
            &qs(format!("{} (*.exe *.app *.rsrc *.bin);;{} (*.dir *.dxr *.mmm);;{} (*)",
                tr!(self.l, "file-load_browse-projector-file-type"),
                tr!(self.l, "file-load_browse-movies-file-type"),
                tr!(self.l, "file-load_browse-all-files-file-type"),
            )),
            NullPtr,
            FileDialogOption::ReadOnly.into());
        if !path_str.is_empty() {
            self.validate_input(&path_str);
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
            qtr!(self.l, "data-dir_browse-title"),
            &self.tabs.options.data_dir.input.text(),
        );
        if !path_str.is_empty() {
            self.tabs.options.data_dir.input.set_text(&path_str);
        }
    }

    #[slot(SlotNoArgs)]
    unsafe fn on_input(self: &Rc<Self>) {
        self.validate_input(&self.file.input.text());
    }

    unsafe fn validate_input(self: &Rc<Self>, chosen_path: &CppBox<QString>) {
        let std_path = chosen_path.to_std_string();
        let is_valid = if chosen_path.is_empty() {
            false
        } else {
            match detect(&std_path) {
                Ok(info) => {
                    match info {
                        FileType::Projector(info, ..) => {
                            self.tabs.info.file_name.set_text(&qs(info.name().unwrap_or(&tr!(self.l, "file-info_unknown-file-name"))));
                            self.tabs.info.kind.set_text(qtr!(
                                self.l,
                                "file-info_projector-file-kind",
                                [
                                    "version" => info.version().to_string(),
                                    "platform" => info.config().platform().to_string()
                                ]
                            ));
                            // TODO: Heuristic detection of character set
                            self.tabs.options.charset.set_current_index(0);
                            true
                        },
                        FileType::Movie(info, ..) if info.kind() != MovieKind::Cast => {
                            let path = Path::new(&std_path);
                            self.tabs.info.file_name.set_text(&qs(path.file_stem().unwrap().to_string_lossy()));
                            self.tabs.info.kind.set_text(qtr!(
                                self.l,
                                "file-info_movie-file-kind",
                                [
                                    "version" => info.version().to_string(),
                                    "kind" => info.kind().to_string()
                                ]
                            ));
                            // TODO: Heuristic detection of character set
                            self.tabs.options.charset.set_current_index(0);
                            true
                        },
                        _ => {
                            self.detection_failure(&tr!(self.l, "file-info_error-cannot-play-cast"), None);
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
            self.file.input.set_text(chosen_path);
            *self.filename.borrow_mut() = Some(std_path);
        } else {
            *self.filename.borrow_mut() = None;
        }

        self.tabs.tabs.set_tab_enabled(self.tabs.options.tab_index, is_valid);
        self.tabs.info.stack.set_current_index(if is_valid { self.tabs.info.loaded_index } else { self.tabs.info.not_loaded_index });
        self.ok_button.set_disabled(!is_valid);
    }
}
