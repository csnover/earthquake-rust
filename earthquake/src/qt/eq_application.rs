use cpp::{cpp, cpp_class};
use cpp_core::{CppDeletable, Ptr, StaticUpcast};
use qt_core::{QBox, QCoreApplicationArgs, QEvent, QObject};
use std::{any::Any, panic, process, sync::Mutex};

// This code is superbad and I am very very sorry, but (1) sometimes panicks
// happen and the user needs to know about it instead of the program silently
// crashing if they don’t have an open console, and (2) Qt says that it is not
// exception-safe and prints a warning if an exception bubbles out of an event
// handler.
struct Unwind {
    report: Mutex<String>,
    old_hook: Box<dyn Fn(&panic::PanicInfo<'_>) + 'static + Sync + Send>,
}
static mut LAST_ERROR: Option<Unwind> = None;
fn catch_unwind<F: FnOnce() -> R + panic::UnwindSafe, R>(f: F) -> std::thread::Result<R> {
    if unsafe { LAST_ERROR.is_some() } {
        eprintln!("catch_unwind reentered");
        process::abort();
    }

    let state = Unwind {
        report: Mutex::new(String::new()),
        old_hook: panic::take_hook(),
    };
    unsafe { LAST_ERROR = Some(state); }
    panic::set_hook(Box::new(|info| {
        #[cfg(feature = "panic_info_message")]
        let report = info.message().map(|m| format!("{}", m));

        #[cfg(not(feature = "panic_info_message"))]
        let report = {
            let mut report = if let Some(payload) = info.payload().downcast_ref::<String>() {
                payload.clone()
            } else if let Some(&payload) = info.payload().downcast_ref::<&str>() {
                payload.to_string()
            } else {
                "Panic".to_string()
            };

            if let Some(location) = info.location() {
                report += &format!(", {}:{}:{}", location.file(), location.line(), location.column());
            }

            report
        };

        unsafe {
            let error = LAST_ERROR.as_mut().unwrap();
            *error.report.get_mut().unwrap() = report;
            (error.old_hook)(info);
        }
    }));
    let result = panic::catch_unwind(f);
    let Unwind { old_hook, report } = unsafe { LAST_ERROR.take() }.unwrap();
    panic::set_hook(old_hook);
    result.map_err(|_| Box::new(report.into_inner().unwrap()) as Box<dyn Any + Send>)
}

pub trait EventReceiver {
    fn error(&mut self, error: Box<dyn Any + Send>) -> bool;
    fn event(&mut self, event: &QEvent) -> bool;
}

cpp!{{
    #include <QtWidgets/QApplication>
    struct TraitPtr { void *a,*b; };
    class EQApplication final : public QApplication {
    public:
        EQApplication(int &argc, char **argv) :
            QApplication(argc, argv),
            _trait() {}

        void setEventReceiver(TraitPtr t) {
            _trait = t;
        }

    protected:
        bool event(QEvent *event) override {
            bool result = false;

            if (_trait.a && _trait.b) {
                result = rust!(EQApplication_event [_trait: &mut dyn EventReceiver as "TraitPtr", event: &QEvent as "QEvent*"] -> bool as "bool" {
                    catch_unwind(panic::AssertUnwindSafe(|| {
                        _trait.event(event)
                    })).unwrap_or_else(|error| _trait.error(error))
                });
            }

            if (result) {
                return true;
            } else {
                return QApplication::event(event);
            }
        }

    private:
        TraitPtr _trait;
    };
}}

cpp_class!(pub(crate) unsafe struct EQApplication as "EQApplication");

// argc and argv are canonical names but trigger the similar_names lint
#[allow(clippy::similar_names)]
impl EQApplication {
    fn new(argc: *mut i32, argv: *mut *mut i8) -> QBox<Self> {
        unsafe {
            QBox::from_raw(cpp!([ argc as "int*", argv as "char**" ] -> *const EQApplication as "EQApplication*" {
                return new EQApplication(*argc, argv);
            }))
        }
    }

    pub fn init<F: FnOnce(Ptr<Self>) -> i32>(f: F) -> ! {
        let exit_code = {
            unsafe {
                let mut args = QCoreApplicationArgs::new();
                let (argc, argv) = args.get();
                let app = EQApplication::new(argc, argv);
                f(app.as_ptr())
            }
        }; // drop `app` and `args`
        process::exit(exit_code)
    }

    pub unsafe fn set_event_receiver(&self, receiver: &dyn EventReceiver) {
        cpp!([ self as "EQApplication*", receiver as "TraitPtr" ] {
            self->setEventReceiver(receiver);
        });
    }
}

impl CppDeletable for EQApplication {
    unsafe fn delete(&self) {
        cpp!([ self as "EQApplication*" ] {
            delete self;
        });
    }
}

impl StaticUpcast<QObject> for EQApplication {
    unsafe fn static_upcast(ptr: Ptr<Self>) -> Ptr<QObject> {
        let raw_ptr = ptr.as_raw_ptr();
        Ptr::from_raw(cpp!([ raw_ptr as "EQApplication*" ] -> *const QObject as "QObject*" {
            return static_cast<QObject*>(raw_ptr);
        }))
    }
}
