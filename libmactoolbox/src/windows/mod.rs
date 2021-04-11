mod manager;

pub use manager::Manager;

use libcommon::newtype_num;
use derive_more::Constructor;

newtype_num! {
    /// A window definition ID.
    ///
    /// A window definition ID is a combination of a `'WDEF'` resource number
    /// plus a variation code, using the form `(16 * res_num) + variation`.
    /// The resource contains the actual window definition function code. Mac
    /// OS includes `'WDEF'` resources 0 and 1; other apps could include their
    /// own.
    ///
    /// The variation codes are arbitrary according to the window definition
    /// being used and are passed as an argument to the window definition
    /// function.
    #[derive(Constructor, Debug)]
    pub struct Kind(i16);
}

impl Kind {
    /// A moveable, sizeable window with no zoom box.
    ///
    /// `documentProc`
    pub const DOCUMENT: Self = Self(0);

    /// A modal dialog box.
    ///
    /// `dBoxProc`
    pub const DIALOG_BOX: Self = Self(1);

    /// A modal dialog box with plain borders.
    ///
    /// `plainDBox`
    pub const PLAIN_DIALOG_BOX: Self = Self(2);

    /// A plain modal dialog box with a drop shadow.
    ///
    /// `altDBoxProc`
    pub const ALTERNATE_DIALOG_BOX: Self = Self(3);

    /// A modeless dialog box with no size box or zoom box.
    ///
    /// `noGrowDocProc`
    pub const NO_GROW_DOCUMENT: Self = Self(4);

    /// A moveable modal dialog box.
    ///
    /// `movableDBoxProc`
    pub const MOVEABLE_DIALOG_BOX: Self = Self(5);

    /// A standard document window with a close box, size box, and zoom box.
    ///
    /// `zoomDocProc`
    pub const ZOOMABLE_DOCUMENT: Self = Self(8);

    /// A document window with a close box and zoom box, but no size box.
    ///
    /// “is virtually never appropriate” —Macintosh Toolbox Essentials 4-10
    ///
    /// `zoomNoGrow`
    pub const ZOOMABLE_NO_GROW_DOCUMENT: Self = Self(12);

    /// A document window with rounded corners.
    ///
    /// `rDocProc`
    pub const ROUNDED_DOCUMENT: Self = Self(16);
}
/// A colour window record.
#[derive(Clone, Copy, Debug, Default)]
pub struct CWindowRecord;
