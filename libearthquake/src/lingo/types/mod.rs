mod actor;
mod cast;
mod symbol;

pub(crate) use actor::Actor;
pub(crate) use symbol::Symbol;
use libmactoolbox::types::MacString;

use libcommon::UnkHnd;
use super::vm::OpCode;

#[derive(Clone, Debug)]
pub(super) struct List(Vec<Variant>);

#[derive(Clone, Debug)]
pub(super) struct Point(List);

#[derive(Clone, Debug)]
pub(super) struct Rect(List);

#[derive(Clone, Debug)]
pub(super) enum Variant {
    Null,
    String(MacString),
    Void,
    XObject(UnkHnd),
    Integer(i32),
    Picture(UnkHnd),
    Object(UnkHnd),
    Symbol(i16),
    // D3Mac used 80-bit floating point, but by the time it got to Windows it
    // was loading 64-bit doubles, so probably f64 is fine. Maybe don’t use
    // this software for scientific calculations, please.
    Float(f64),
}

pub(super) enum Error {}

struct TellHandle;

// Allow unused variables for the sake of API documentation.
// https://github.com/rust-lang/rust/issues/26487
#[allow(unused_variables)]
trait VmObject {
    /// Returns the symbol for the kind of the object.
    fn symbol(&self) -> Symbol;

    /// Calls a function with the given name on this object.
    fn function(&mut self, function_name: Symbol) -> bool { false }

    /// Forwards a call to a handler contained in another parent script (class).
    fn send_ancestor(&mut self, handler: Symbol) -> bool { false }

    /// Deletes the object.
    fn free(&mut self) {}

    /// Generates a debug representation of the object.
    ///
    /// In original Lingo, this received a callback function which was called
    /// periodically to send the debug output with a signature of
    /// `void (*cb)(void *obj, char *str)`, and the `obj` was a parameter.
    /// Hopefully that is not necessary here.
    fn debug_print(&self, sink: &mut impl std::io::Write) {}

    /// Gets the object’s identifier as an integer.
    fn to_integer(&self) -> Option<i32> { None }

    /// Gets or sets a property of the object.
    fn property(&mut self, is_set: bool, property_name: Symbol, value: &mut Variant) -> bool { false }

    /// Begins a tell block for the object, if the object can receive messages.
    fn tell_start(&mut self) -> Option<TellHandle> { None }

    /// Ends a tell block for the object.
    fn tell_end(&mut self, handle: TellHandle) {}

    /// Runs a single tell action.
    fn tell(&self, verb: Symbol) -> bool { false }

    /// Executes an operation on the object by VM opcode.
    fn call_op(&self, op: OpCode) -> bool { false }

    // TODO: Not at all ready to do this yet.
    // /// Writes the object to a RIFF file.
    // fn write(&self, riff: &mut Riff) {}
}
