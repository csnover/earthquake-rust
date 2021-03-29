use binrw::io::{Cursor, Read, Seek, SeekFrom};
use libcommon::io::SharedStream;

#[test]
fn test_substream() {
    const IN_START: u16 = 2;
    const OUT_START: u16 = 1;
    const IN_SIZE: u16 = 4;
    let mut data = Cursor::new(vec![ 0, 1, 2, 3, 4, 5, 6, 7, 8, 9 ]);
    let mut out = Vec::with_capacity(IN_SIZE.into());
    let mut out2 = Vec::with_capacity(IN_SIZE.into());

    data.seek(SeekFrom::Start(IN_SIZE.into())).unwrap();

    let mut stream = SharedStream::with_bounds(data, IN_START.into(), (IN_START + IN_SIZE).into());
    stream.seek(SeekFrom::Start(OUT_START.into())).unwrap();
    assert_eq!(stream.seek(SeekFrom::Current(0)).unwrap(), OUT_START.into());

    let mut stream2 = stream.clone();
    let size = stream.read_to_end(&mut out).unwrap();
    let size2 = stream2.read_to_end(&mut out2).unwrap();
    assert_eq!(size, (IN_SIZE - OUT_START).into());
    assert_eq!(size, size2);
    assert_eq!(out[0..(IN_SIZE - OUT_START).into()], [ 3, 4, 5 ]);
    assert_eq!(out, out2);
}
