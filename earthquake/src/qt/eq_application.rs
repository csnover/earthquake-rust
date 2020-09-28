use cpp::{cpp, cpp_class};
use cpp_core::{CppDeletable, Ptr, StaticUpcast};
use qt_core::{QBox, QCoreApplicationArgs, QEvent, QObject};
use std::process;

pub trait EventReceiver {
    fn event(&mut self, event: &QEvent) -> bool;
}

cpp!{{
    #include <QtWidgets/QApplication>
    struct TraitPtr { void *a,*b; };
    class EQApplication : public QApplication {
    public:
        EQApplication(int &argc, char **argv) :
            QApplication(argc, argv),
            _trait() {}

        void setEventReceiver(TraitPtr t) {
            _trait = t;
        }

    protected:
        bool event(QEvent *e) override {
            bool result = false;

            if (_trait.a && _trait.b) {
                result = rust!(EQApplication_event [_trait: &mut dyn EventReceiver as "TraitPtr", e: &QEvent as "QEvent*"] -> bool as "bool" {
                    _trait.event(e)
                });
            }

            if (result) {
                return true;
            } else {
                return QApplication::event(e);
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
