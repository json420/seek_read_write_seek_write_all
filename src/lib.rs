#![feature(core_io_borrowed_buf)]
#![feature(seek_read_exact_seek_write_all)]

use std::os::windows::fs::FileExt;
use tempfile;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File, OpenOptions};
    use std::io::{self, ErrorKind, Read, Seek, SeekFrom, Write};

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
    fn file_test_io_seek_read_exact_write_all() {
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
        // Test when seek_read(), seek_write() return Ok(0)
        struct MockFile {
            expected_offset: u64,
        }

        impl FileExt for MockFile {
            fn seek_read(&self, _buf: &mut [u8], offset: u64) -> io::Result<usize> {
                assert_eq!(offset, self.expected_offset);
                Ok(0)
            }

            fn seek_write(&self, _buf: &[u8], offset: u64) -> io::Result<usize> {
                assert_eq!(offset, self.expected_offset);
                Ok(0)
            }
        }

        {
            let mock_file = MockFile { expected_offset: 0 };
            let mut buf = [0; 256];
            assert_eq!(
                mock_file.seek_read_exact(&mut buf, 0).unwrap_err().kind(),
                io::ErrorKind::UnexpectedEof
            );
            assert_eq!(
                mock_file.seek_write_all(&buf, 0).unwrap_err().kind(),
                io::ErrorKind::WriteZero
            );
        }

        {
            let mock_file = MockFile {
                expected_offset: 420,
            };
            let mut buf = [0; 256];
            assert_eq!(
                mock_file.seek_read_exact(&mut buf, 420).unwrap_err().kind(),
                io::ErrorKind::UnexpectedEof
            );
            assert_eq!(
                mock_file.seek_write_all(&buf, 420).unwrap_err().kind(),
                io::ErrorKind::WriteZero
            );
        }
    }

    #[test]
    #[cfg(windows)]
    fn file_test_windows_fileext_trait_case_3() {
        // Test that Err other than io::ErrorKind::Interrupted are propagated up.
        struct MockFile {
            expected_offset: u64,
        }

        impl FileExt for MockFile {
            fn seek_read(&self, _buf: &mut [u8], offset: u64) -> io::Result<usize> {
                assert_eq!(offset, self.expected_offset);
                Err(io::Error::new(io::ErrorKind::PermissionDenied, "seek_read"))
            }

            fn seek_write(&self, _buf: &[u8], offset: u64) -> io::Result<usize> {
                assert_eq!(offset, self.expected_offset);
                Err(io::Error::new(
                    io::ErrorKind::ConnectionRefused,
                    "seek_write",
                ))
            }
        }

        {
            let mock_file = MockFile { expected_offset: 0 };
            let mut buf = [0; 256];
            assert_eq!(
                mock_file.seek_read_exact(&mut buf, 0).unwrap_err().kind(),
                io::ErrorKind::PermissionDenied
            );
            assert_eq!(
                mock_file.seek_write_all(&buf, 0).unwrap_err().kind(),
                io::ErrorKind::ConnectionRefused
            );
        }

        {
            let mock_file = MockFile {
                expected_offset: 420,
            };
            let mut buf = [0; 256];
            assert_eq!(
                mock_file.seek_read_exact(&mut buf, 420).unwrap_err().kind(),
                io::ErrorKind::PermissionDenied
            );
            assert_eq!(
                mock_file.seek_write_all(&buf, 420).unwrap_err().kind(),
                io::ErrorKind::ConnectionRefused
            );
        }
        // FIXME: Cover io::ErrorKind::Interrupted, but don't infinite loop ;)
    }

    #[test]
    #[cfg(windows)]
    fn file_test_windows_fileext_trait_case_4() {
        const MSG: &[u8] =
            b"The Rust programming language helps you write faster, more reliable software.";

        // Test when the entire read or write is satisfied by only one call to seek_read() or
        // seek_write(), respectively.
        struct MockFile {
            expected_offset: u64,
        }

        impl FileExt for MockFile {
            fn seek_read(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
                assert_eq!(offset, self.expected_offset);
                assert_eq!(buf.len(), MSG.len());
                assert_eq!(buf, &[0; MSG.len()]);
                buf.copy_from_slice(MSG);
                Ok(MSG.len())
            }

            fn seek_write(&self, buf: &[u8], offset: u64) -> io::Result<usize> {
                assert_eq!(offset, self.expected_offset);
                assert_eq!(buf.len(), MSG.len());
                assert_eq!(buf, MSG);
                Ok(MSG.len())
            }
        }

        {
            let mock_file = MockFile { expected_offset: 0 };
            let mut buf = [0; MSG.len()];
            check!(mock_file.seek_read_exact(&mut buf, 0));
            assert_eq!(&buf, MSG);
            check!(mock_file.seek_write_all(&buf, 0));
        }

        {
            let mock_file = MockFile {
                expected_offset: 420,
            };
            let mut buf = [0; MSG.len()];
            check!(mock_file.seek_read_exact(&mut buf, 420));
            assert_eq!(&buf, MSG);
            check!(mock_file.seek_write_all(&buf, 420));
        }
    }

    #[test]
    fn file_test_windows_fileext_trait_case_5() {
        const MSG: &[u8] =
        b"Rust is for students and those who are interested in learning about systems concepts.";

        // Test pathological case where seek_read(), seek_write() only do 1 byte per call, return Ok(1)
        struct MockFile {
            base_offset: u64,
        }

        impl FileExt for MockFile {
            fn seek_read(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
                let offset = (offset - self.base_offset) as usize;
                buf[0..1].copy_from_slice(&MSG[offset..offset + 1]);
                Ok(1)
            }

            fn seek_write(&self, buf: &[u8], offset: u64) -> io::Result<usize> {
                let offset = (offset - self.base_offset) as usize;
                assert_eq!(buf[0..1], MSG[offset..offset + 1]);
                Ok(1)
            }
        }

        {
            let mock_file = MockFile { base_offset: 0 };
            let mut buf = [0; MSG.len()];
            check!(mock_file.seek_read_exact(&mut buf, 0));
            assert_eq!(&buf, MSG);
            check!(mock_file.seek_write_all(&buf, 0));
        }

        {
            let mock_file = MockFile { base_offset: 420 };
            let mut buf = [0; MSG.len()];
            check!(mock_file.seek_read_exact(&mut buf, 420));
            assert_eq!(&buf, MSG);
            check!(mock_file.seek_write_all(&buf, 420));
        }
    }
}
