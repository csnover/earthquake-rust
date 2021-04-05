use binrw::io::Cursor;
use byteorder::{BigEndian, LittleEndian};
use libmactoolbox::resources::{OsType, OsTypeReadExt, ResourceId};

#[test]
fn basic() {
    println!("{:?}", ResourceId::new(b"HELO", 123));
    let id = ResourceId::new(b"HELO", 123);
    assert_eq!(id.os_type(), OsType::new(*b"HELO"));
    assert_eq!(id.id(), 123);
}

#[test]
fn os_type_primitive() {
    let os_type = OsType::new(*b"HeLO");
    assert_eq!(format!("{}", os_type), "HeLO");
    assert_eq!(format!("{:?}", os_type), "OSType(HeLO)");
}

#[test]
fn os_type_read() {
    let mut c = Cursor::new(b"HeLOOLeH");
    assert_eq!(c.read_os_type::<BigEndian>().unwrap(), OsType::new(*b"HeLO"));
    assert_eq!(c.read_os_type::<LittleEndian>().unwrap(), OsType::new(*b"HeLO"));
}

#[test]
fn os_type_from_u32() {
    let os_type = 0x48_65_4c_4f;
    assert_eq!(OsType::from(os_type), OsType::new(*b"HeLO"));
}
