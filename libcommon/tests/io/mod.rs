mod shared_stream;

#[test]
fn take_seek() {
    use binrw::io::{Cursor, Read, Seek, SeekFrom};
    use libcommon::TakeSeekExt;
    let mut cursor = Cursor::new(b"hello world");
    {
        let mut buf = [0; 5];
        let mut take = cursor.by_ref().take_seek(5);
        take.read_exact(&mut buf).unwrap();
        assert_eq!(&buf[..], b"hello");
    }

    {
        let mut buf = [0; 5];
        let mut take = cursor.by_ref().take_seek(5);
        assert_eq!(take.seek(SeekFrom::Current(0)).unwrap(), 5);
        take.read_exact(&mut buf).unwrap();
        assert_eq!(&buf[..], b" worl");
        assert_eq!(take.seek(SeekFrom::Current(0)).unwrap(), 10);
    }

    {
        let mut buf = [0; 5];
        let mut take = cursor.by_ref().take_seek(5);
        assert_eq!(take.read(&mut buf).unwrap(), 1);
        assert_eq!(buf[0], b'd');
        assert_eq!(take.seek(SeekFrom::Current(0)).unwrap(), 11);
    }

    {
        let mut buf = [0; 5];
        let mut take = cursor.by_ref().take_seek(5);
        take.seek(SeekFrom::Current(-1)).unwrap();
        assert_eq!(take.read(&mut buf).unwrap(), 1);
        assert_eq!(buf[0], b'd');
        assert_eq!(take.read(&mut buf).unwrap(), 0);
    }

    {
        let mut buf = [0; 5];
        let mut take = cursor.by_ref().take_seek(5);
        take.seek(SeekFrom::Start(0)).unwrap();
        assert_eq!(take.read(&mut buf).unwrap(), 5);
        assert_eq!(&buf[..], b"hello");
    }

    {
        let mut buf = [0; 5];
        let mut take = cursor.by_ref().take_seek(5);
        take.seek(SeekFrom::End(0)).unwrap();
        assert_eq!(take.seek(SeekFrom::Current(0)).unwrap(), 10);
        assert_eq!(take.read(&mut buf).unwrap(), 0);
    }

    {
        let mut buf = [0; 5];
        let mut take = cursor.by_ref().take_seek(5);
        take.seek(SeekFrom::Start(1000)).unwrap();
        assert_eq!(take.read(&mut buf).unwrap(), 0);
    }
}
