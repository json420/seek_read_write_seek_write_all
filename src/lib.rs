#![feature(core_io_borrowed_buf)]
#![feature(seek_read_exact_seek_write_all)]

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;
    use getrandom;
    use std::fs::{self, File, OpenOptions};
    use std::io::{self, BorrowedBuf, ErrorKind, Read, Seek, SeekFrom, Write};
    use std::mem::MaybeUninit;
    use tempfile;

    macro_rules! check {
        ($e:expr) => {
            match $e {
                Ok(t) => t,
                Err(e) => panic!("{} failed with: {e}", stringify!($e)),
            }
        };
    }

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

    #[test]
    fn test_seek_read_exact_seek_write_all() {
        use std::os::windows::fs::FileExt;

        // Make sure windows FileExt.seek_read_exact(), .seek_write_all() are available
        let mut file = tempfile::tempfile().unwrap();
        let mut data = [0; 420];
        getrandom::fill(&mut data).unwrap();
        let data = data;
        file.seek_write_all(&data, 0).unwrap();
        let mut buf = [0; 42];
        file.seek_read_exact(&mut buf, 22).unwrap();
        assert_eq!(&buf, &data[22..64]);
        assert_eq!(file.stream_position().unwrap(), 64);

        // Make sure .seek_read_buf_exact() is available
        let mut buf: [MaybeUninit<u8>; 11] = [MaybeUninit::uninit(); 11];
        let mut buf = BorrowedBuf::from(buf.as_mut_slice());
        file.seek_read_buf_exact(buf.unfilled(), 69).unwrap();
        assert_eq!(buf.filled(), &data[69..80]);
        assert_eq!(file.stream_position().unwrap(), 80);
    }

    #[test]
    fn file_test_io_seek_read_exact_write_all() {
        use std::os::windows::fs::FileExt;

        let tmpdir = tempfile::TempDir::new().unwrap();
        let filename = tmpdir
            .path()
            .join("file_rt_io_file_test_seek_read_exact_write_all.txt");
        let mut buf = [0; 256];
        let write1 = "asdf";
        let write2 = "qwer-";
        let write3 = "-zxcv";
        let content = "qwer-asdf-zxcv";
        {
            let oo = OpenOptions::new()
                .create_new(true)
                .write(true)
                .read(true)
                .clone();
            let mut rw = check!(oo.open(&filename));
            check!(rw.seek_write_all(write1.as_bytes(), 5));
            assert_eq!(check!(rw.stream_position()), 9);
            check!(rw.seek_read_exact(&mut buf[..write1.len()], 5));
            assert_eq!(str::from_utf8(&buf[..write1.len()]), Ok(write1));
            assert_eq!(check!(rw.stream_position()), 9);
            assert_eq!(check!(rw.seek(SeekFrom::Start(0))), 0);
            assert_eq!(check!(rw.write(write2.as_bytes())), write2.len());
            assert_eq!(check!(rw.stream_position()), 5);
            assert_eq!(check!(rw.read(&mut buf)), write1.len());
            assert_eq!(str::from_utf8(&buf[..write1.len()]), Ok(write1));
            assert_eq!(check!(rw.stream_position()), 9);
            check!(rw.seek_read_exact(&mut buf[..write2.len()], 0));
            assert_eq!(str::from_utf8(&buf[..write2.len()]), Ok(write2));
            assert_eq!(check!(rw.stream_position()), 5);
            check!(rw.seek_write_all(write3.as_bytes(), 9));
            assert_eq!(check!(rw.stream_position()), 14);
        }
        {
            let mut read = check!(File::open(&filename));
            check!(read.seek_read_exact(&mut buf[..content.len()], 0));
            assert_eq!(str::from_utf8(&buf[..content.len()]), Ok(content));
            assert_eq!(check!(read.stream_position()), 14);
            assert_eq!(check!(read.seek(SeekFrom::End(-5))), 9);
            check!(read.seek_read_exact(&mut buf[..content.len()], 0));
            assert_eq!(str::from_utf8(&buf[..content.len()]), Ok(content));
            assert_eq!(check!(read.stream_position()), 14);
            assert_eq!(check!(read.seek(SeekFrom::End(-5))), 9);
            assert_eq!(check!(read.read(&mut buf)), write3.len());
            assert_eq!(str::from_utf8(&buf[..write3.len()]), Ok(write3));
            assert_eq!(check!(read.stream_position()), 14);
            check!(read.seek_read_exact(&mut buf[..content.len()], 0));
            assert_eq!(str::from_utf8(&buf[..content.len()]), Ok(content));
            assert_eq!(check!(read.stream_position()), 14);
            assert!(read.seek_read_exact(&mut buf, 14).is_err());
            assert!(read.seek_read_exact(&mut buf, 15).is_err());
        }
        check!(fs::remove_file(&filename));
    }

    #[test]
    #[cfg(windows)]
    fn file_test_windows_fileext_trait_case_1() {
        use std::os::windows::fs::FileExt;

        // Test when seek_read_exact(), seek_write_all() are called with empty buffers.
        struct MockFile {}

        impl FileExt for MockFile {
            fn seek_read(&self, _buf: &mut [u8], _offset: u64) -> io::Result<usize> {
                panic!("should not be called");
            }

            fn seek_write(&self, _buf: &[u8], _offset: u64) -> io::Result<usize> {
                panic!("should not be called");
            }
        }

        let mock_file = MockFile {};
        check!(mock_file.seek_read_exact(&mut [], 0));
        check!(mock_file.seek_write_all(&[], 0));
        check!(mock_file.seek_read_exact(&mut [], 420));
        check!(mock_file.seek_write_all(&[], 420));
    }

    #[test]
    #[cfg(windows)]
    fn file_test_windows_fileext_trait_case_2() {
        use std::cell::RefCell;
        use std::os::windows::fs::FileExt;

        // Test when seek_read(), seek_write() return Ok(0)
        struct MockFile {
            expected_offset: u64,
            read_calls: RefCell<usize>,
            write_calls: RefCell<usize>,
        }

        impl MockFile {
            fn new(expected_offset: u64) -> Self {
                Self {
                    expected_offset,
                    read_calls: RefCell::new(0),
                    write_calls: RefCell::new(0),
                }
            }
        }

        impl FileExt for MockFile {
            fn seek_read(&self, _buf: &mut [u8], offset: u64) -> io::Result<usize> {
                *self.read_calls.borrow_mut() += 1;
                assert_eq!(offset, self.expected_offset);
                Ok(0)
            }

            fn seek_write(&self, _buf: &[u8], offset: u64) -> io::Result<usize> {
                *self.write_calls.borrow_mut() += 1;
                assert_eq!(offset, self.expected_offset);
                Ok(0)
            }
        }

        let mut buf = [0; 256];
        for offset in [0, 1, 420] {
            {
                let mock_file = MockFile::new(offset);
                assert_eq!(
                    mock_file
                        .seek_read_exact(&mut buf, offset)
                        .unwrap_err()
                        .kind(),
                    ErrorKind::UnexpectedEof
                );
                assert_eq!(*mock_file.read_calls.borrow(), 1);
                assert_eq!(*mock_file.write_calls.borrow(), 0);
            }
            {
                let mock_file = MockFile::new(offset);
                assert_eq!(
                    mock_file.seek_write_all(&buf, offset).unwrap_err().kind(),
                    ErrorKind::WriteZero
                );
                assert_eq!(*mock_file.read_calls.borrow(), 0);
                assert_eq!(*mock_file.write_calls.borrow(), 1);
            }
        }
    }

    #[test]
    #[cfg(windows)]
    fn file_test_windows_fileext_trait_case_3() {
        use std::cell::RefCell;
        use std::os::windows::fs::FileExt;

        // Test that Err are propagated up.
        struct MockFile {
            expected_offset: u64,
            read_calls: RefCell<usize>,
            write_calls: RefCell<usize>,
        }

        impl MockFile {
            fn new(expected_offset: u64) -> Self {
                Self {
                    expected_offset,
                    read_calls: RefCell::new(0),
                    write_calls: RefCell::new(0),
                }
            }
        }

        impl FileExt for MockFile {
            fn seek_read(&self, _buf: &mut [u8], offset: u64) -> io::Result<usize> {
                *self.read_calls.borrow_mut() += 1;
                assert_eq!(offset, self.expected_offset);
                Err(io::Error::new(ErrorKind::PermissionDenied, "seek_read"))
            }

            fn seek_write(&self, _buf: &[u8], offset: u64) -> io::Result<usize> {
                *self.write_calls.borrow_mut() += 1;
                assert_eq!(offset, self.expected_offset);
                Err(io::Error::new(ErrorKind::ConnectionRefused, "seek_write"))
            }
        }

        let mut buf = [0; 256];
        for offset in [0, 1, 420] {
            {
                let mock_file = MockFile::new(offset);
                assert_eq!(
                    mock_file
                        .seek_read_exact(&mut buf, offset)
                        .unwrap_err()
                        .kind(),
                    ErrorKind::PermissionDenied
                );
                assert_eq!(*mock_file.read_calls.borrow(), 1);
                assert_eq!(*mock_file.write_calls.borrow(), 0);
            }
            {
                let mock_file = MockFile::new(offset);
                assert_eq!(
                    mock_file.seek_write_all(&buf, offset).unwrap_err().kind(),
                    ErrorKind::ConnectionRefused
                );
                assert_eq!(*mock_file.read_calls.borrow(), 0);
                assert_eq!(*mock_file.write_calls.borrow(), 1);
            }
        }
    }

    // This should fail now, but pass once there is a suitable rust nightly build
    //#[test]
    #[cfg(windows)]
    fn file_test_windows_fileext_trait_case_4() {
        use std::cell::RefCell;
        use std::os::windows::fs::FileExt;

        // Test that Err ErrorKind::Interrupted are propigated up (unlike Unix versions)
        struct MockFile {
            expected_offset: u64,
            read_calls: RefCell<usize>,
            write_calls: RefCell<usize>,
        }

        impl MockFile {
            fn new(expected_offset: u64) -> Self {
                Self {
                    expected_offset,
                    read_calls: RefCell::new(0),
                    write_calls: RefCell::new(0),
                }
            }
        }

        impl FileExt for MockFile {
            fn seek_read(&self, _buf: &mut [u8], offset: u64) -> io::Result<usize> {
                *self.read_calls.borrow_mut() += 1;
                assert_eq!(offset, self.expected_offset);
                Err(io::Error::new(ErrorKind::Interrupted, "seek_read"))
            }

            fn seek_write(&self, _buf: &[u8], offset: u64) -> io::Result<usize> {
                *self.write_calls.borrow_mut() += 1;
                assert_eq!(offset, self.expected_offset);
                Err(io::Error::new(ErrorKind::Interrupted, "seek_write"))
            }
        }

        let mut buf = [0; 256];
        for offset in [0, 1, 420] {
            {
                let mock_file = MockFile::new(offset);
                assert_eq!(
                    mock_file
                        .seek_read_exact(&mut buf, offset)
                        .unwrap_err()
                        .kind(),
                    ErrorKind::Interrupted
                );
                assert_eq!(*mock_file.read_calls.borrow(), 1);
                assert_eq!(*mock_file.write_calls.borrow(), 0);
            }
            {
                let mock_file = MockFile::new(offset);
                assert_eq!(
                    mock_file.seek_write_all(&buf, offset).unwrap_err().kind(),
                    ErrorKind::Interrupted
                );
                assert_eq!(*mock_file.read_calls.borrow(), 0);
                assert_eq!(*mock_file.write_calls.borrow(), 1);
            }
        }
    }

    #[test]
    #[cfg(windows)]
    fn file_test_windows_fileext_trait_case_5() {
        use std::cell::RefCell;
        use std::os::windows::fs::FileExt;

        const MSG: &[u8] =
            b"The Rust programming language helps you write faster, more reliable software.";

        // Test when the entire read or write is satisfied by only one call to seek_read() or
        // seek_write(), respectively.
        struct MockFile {
            expected_offset: u64,
            read_calls: RefCell<usize>,
            write_calls: RefCell<usize>,
        }

        impl MockFile {
            fn new(expected_offset: u64) -> Self {
                Self {
                    expected_offset,
                    read_calls: RefCell::new(0),
                    write_calls: RefCell::new(0),
                }
            }
        }

        impl FileExt for MockFile {
            fn seek_read(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
                *self.read_calls.borrow_mut() += 1;
                assert_eq!(offset, self.expected_offset);
                assert_eq!(buf.len(), MSG.len());
                assert_eq!(buf, &[0; MSG.len()]);
                buf.copy_from_slice(MSG);
                Ok(MSG.len())
            }

            fn seek_write(&self, buf: &[u8], offset: u64) -> io::Result<usize> {
                *self.write_calls.borrow_mut() += 1;
                assert_eq!(offset, self.expected_offset);
                assert_eq!(buf.len(), MSG.len());
                assert_eq!(buf, MSG);
                Ok(MSG.len())
            }
        }

        for offset in [0, 1, 420] {
            {
                let mock_file = MockFile::new(offset);
                let mut buf = [0; MSG.len()];
                check!(mock_file.seek_read_exact(&mut buf, offset));
                assert_eq!(&buf, MSG);
                assert_eq!(*mock_file.read_calls.borrow(), 1);
                assert_eq!(*mock_file.write_calls.borrow(), 0);
            }
            {
                let mock_file = MockFile::new(offset);
                check!(mock_file.seek_write_all(MSG, offset));
                assert_eq!(*mock_file.read_calls.borrow(), 0);
                assert_eq!(*mock_file.write_calls.borrow(), 1);
            }
        }
    }

    #[test]
    fn file_test_windows_fileext_trait_case_6() {
        use std::cell::RefCell;
        use std::os::windows::fs::FileExt;

        const MSG: &[u8] =
        b"Rust is for students and those who are interested in learning about systems concepts.";

        // Test pathological case where seek_read(), seek_write() only do 1 byte per call, return Ok(1)
        struct MockFile {
            base_offset: u64,
            read_calls: RefCell<usize>,
            write_calls: RefCell<usize>,
        }

        impl MockFile {
            fn new(base_offset: u64) -> Self {
                Self {
                    base_offset,
                    read_calls: RefCell::new(0),
                    write_calls: RefCell::new(0),
                }
            }
        }

        impl FileExt for MockFile {
            fn seek_read(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
                *self.read_calls.borrow_mut() += 1;
                let offset = (offset - self.base_offset) as usize;
                buf[0..1].copy_from_slice(&MSG[offset..offset + 1]);
                Ok(1)
            }

            fn seek_write(&self, buf: &[u8], offset: u64) -> io::Result<usize> {
                *self.write_calls.borrow_mut() += 1;
                let offset = (offset - self.base_offset) as usize;
                assert_eq!(buf[0..1], MSG[offset..offset + 1]);
                Ok(1)
            }
        }

        for offset in [0, 1, 420] {
            {
                let mock_file = MockFile::new(offset);
                let mut buf = [0; MSG.len()];
                check!(mock_file.seek_read_exact(&mut buf, offset));
                assert_eq!(&buf, MSG);
                assert_eq!(*mock_file.read_calls.borrow(), MSG.len());
                assert_eq!(*mock_file.write_calls.borrow(), 0);
            }
            {
                let mock_file = MockFile::new(offset);
                check!(mock_file.seek_write_all(MSG, offset));
                assert_eq!(*mock_file.read_calls.borrow(), 0);
                assert_eq!(*mock_file.write_calls.borrow(), MSG.len());
            }
        }
    }

    #[test]
    fn file_test_windows_fileext_trait_case_7() {
        use std::cell::RefCell;
        use std::os::windows::fs::FileExt;

        const MSG: &[u8] =
        b"The Rustonomicon digs into all the awful details that you need to understand when writing Unsafe Rust programs.";

        // Test extra pathological case where seek_read(), seek_write() return an `ErrorKind::Interrupted`
        // `Err()` on even calls (starting with the first) and return `Ok(1)` on odd calls, only
        // doing one byte of work for 2 calls.
        struct MockFile {
            base_offset: u64,
            read_calls: RefCell<usize>,
            write_calls: RefCell<usize>,
        }

        impl MockFile {
            fn new(base_offset: u64) -> Self {
                Self {
                    base_offset,
                    read_calls: RefCell::new(0),
                    write_calls: RefCell::new(0),
                }
            }
        }

        impl FileExt for MockFile {
            fn seek_read(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
                *self.read_calls.borrow_mut() += 1;
                if (*self.read_calls.borrow()).is_multiple_of(2) {
                    let offset = (offset - self.base_offset) as usize;
                    buf[0..1].copy_from_slice(&MSG[offset..offset + 1]);
                    Ok(1)
                } else {
                    Err(io::Error::new(ErrorKind::Interrupted, "interrupting read"))
                }
            }

            fn seek_write(&self, buf: &[u8], offset: u64) -> io::Result<usize> {
                *self.write_calls.borrow_mut() += 1;
                if (*self.write_calls.borrow()).is_multiple_of(2) {
                    let offset = (offset - self.base_offset) as usize;
                    assert_eq!(buf[0..1], MSG[offset..offset + 1]);
                    Ok(1)
                } else {
                    Err(io::Error::new(ErrorKind::Interrupted, "interrupting write"))
                }
            }
        }

        for offset in [0, 1, 420] {
            {
                let mock_file = MockFile::new(offset);
                let mut buf = [0; MSG.len()];
                check!(mock_file.seek_read_exact(&mut buf, offset));
                assert_eq!(&buf, MSG);
                assert_eq!(*mock_file.read_calls.borrow(), MSG.len() * 2);
                assert_eq!(*mock_file.write_calls.borrow(), 0);
            }
            {
                let mock_file = MockFile::new(offset);
                check!(mock_file.seek_write_all(MSG, offset));
                assert_eq!(*mock_file.read_calls.borrow(), 0);
                assert_eq!(*mock_file.write_calls.borrow(), MSG.len() * 2);
            }
        }
    }

    #[test]
    fn test_seek_read_buf_exact() {
        use std::os::windows::fs::FileExt;

        let tmpdir = tempfile::TempDir::new().unwrap();
        let filename = tmpdir
            .path()
            .join("file_rt_io_file_test_seek_read_buf_exact.txt");
        {
            let oo = OpenOptions::new()
                .create_new(true)
                .write(true)
                .read(true)
                .clone();
            let mut file = check!(oo.open(&filename));
            check!(file.write_all(b"0123456789"));
        }
        {
            let mut file = check!(File::open(&filename));
            let mut buf: [MaybeUninit<u8>; 5] = [MaybeUninit::uninit(); 5];
            let mut buf = BorrowedBuf::from(buf.as_mut_slice());

            // Exact read
            check!(file.seek_read_buf_exact(buf.unfilled(), 2));
            assert_eq!(buf.filled(), b"23456");
            assert_eq!(check!(file.stream_position()), 7);

            // Already full
            check!(file.seek_read_buf_exact(buf.unfilled(), 3));
            assert_eq!(check!(file.stream_position()), 7);
            check!(file.seek_read_buf_exact(buf.unfilled(), 10)); // No call to seek_read()
            assert_eq!(buf.filled(), b"23456");
            assert_eq!(check!(file.stream_position()), 7);

            // Non-empty exact read past eof fails
            let err = file
                .seek_read_buf_exact(buf.clear().unfilled(), 6)
                .unwrap_err();
            assert_eq!(err.kind(), ErrorKind::UnexpectedEof);
            assert_eq!(check!(file.stream_position()), 10);
        }
        check!(fs::remove_file(&filename));
    }
}
