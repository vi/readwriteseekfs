extern crate fuse;
extern crate libc;
extern crate time;
use self::fuse::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyEmpty, ReplyEntry, ReplyWrite,
    Request,
};
use self::time::Timespec;
use std::ffi::OsStr;
use std::io::{Error, ErrorKind, Read, Result, Seek, SeekFrom, Write};

const CREATE_TIME: Timespec = Timespec {
    sec: 1534631479,
    nsec: 0,
}; //FIXME

const TTL: Timespec = Timespec { sec: 9999, nsec: 0 };

use self::libc::{c_int,EROFS};
fn errmap(e: Error) -> c_int {
    use self::libc::*;
    use ErrorKind::*;
    // TODO parse Other's Display and derive more error codes
    match e.kind() {
        NotFound => ENOENT,
        PermissionDenied => EACCES,
        ConnectionRefused => ECONNREFUSED,
        ConnectionReset => ECONNREFUSED,
        ConnectionAborted => ECONNABORTED,
        NotConnected => ENOTCONN,
        AddrInUse => EADDRINUSE,
        AddrNotAvailable => EADDRNOTAVAIL,
        BrokenPipe => EPIPE,
        AlreadyExists => EEXIST,
        WouldBlock => EWOULDBLOCK,
        InvalidInput => EINVAL,
        InvalidData => EINVAL,
        TimedOut => ETIMEDOUT,
        WriteZero => EINVAL,
        UnexpectedEof => EINVAL,
        _ => EINVAL,
    }
}

trait MyReadEx: Read {
    // Based on https://doc.rust-lang.org/src/std/io/mod.rs.html#620
    fn read_exact2(&mut self, mut buf: &mut [u8]) -> ::std::io::Result<usize> {
        let mut successfully_read = 0;
        while !buf.is_empty() {
            match self.read(buf) {
                Ok(0) => break,
                Ok(n) => {
                    successfully_read += n;
                    let tmp = buf;
                    buf = &mut tmp[n..];
                }
                Err(ref e) if e.kind() == ::std::io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        Ok(successfully_read)
    }
}
impl<T: Read> MyReadEx for T {}

trait MyWriteEx: Write {
    fn write_all2(&mut self, mut buf: &[u8]) -> Result<usize> {
        let mut successfully_written = 0;
        while !buf.is_empty() {
            match self.write(buf) {
                Ok(0) => return Ok(successfully_written),
                Ok(n) => {
                    successfully_written += n;
                    buf = &buf[n..];
                }
                Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        Ok(successfully_written)
    }
}
impl<T: Write> MyWriteEx for T {}

pub struct ReadSeekFs<F: Read + Seek> {
    file: F,
    fa: FileAttr,
}

impl<F> ReadSeekFs<F>
where
    F: Read + Seek,
{
    pub fn new(mut f: F, bs: usize) -> Result<ReadSeekFs<F>> {
        let len = f.seek(SeekFrom::End(0))?;
        let blocks = ((len - 1) / (bs as u64)) + 1;

        Ok(ReadSeekFs {
            file: f,
            fa: FileAttr {
                ino: 1,
                size: len,
                blocks: blocks,
                atime: CREATE_TIME,
                mtime: CREATE_TIME,
                ctime: CREATE_TIME,
                crtime: CREATE_TIME,
                kind: FileType::RegularFile,
                perm: 0o644,
                nlink: 1,
                uid: 0,
                gid: 0,
                rdev: 0,
                flags: 0,
            },
        })
    }

    fn seek(&mut self, offset: i64) -> Result<()> {
        if offset < 0 {
            Err(ErrorKind::InvalidInput)?;
        }
        self.file.seek(SeekFrom::Start(offset as u64))?;
        Ok(())
    }
    fn seek_and_read(&mut self, offset: i64, size: usize) -> Result<Vec<u8>> {
        self.seek(offset)?;
        let mut buf = vec![0; size as usize];
        let ret = self.file.read_exact2(&mut buf)?;
        buf.truncate(ret);
        Ok(buf)
    }
}

impl<F> Filesystem for ReadSeekFs<F>
where
    F: Read + Seek,
{
    fn lookup(&mut self, _req: &Request, _parent: u64, _name: &OsStr, reply: ReplyEntry) {
        reply.entry(&TTL, &self.fa, 0);
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        reply.attr(&TTL, &self.fa);
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        reply: ReplyData,
    ) {
        match self.seek_and_read(offset, size as usize) {
            Ok(buf) => reply.data(buf.as_slice()),
            Err(e) => reply.error(errmap(e)),
        }
    }
    
    fn write(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _flags: u32,
        reply: ReplyWrite,
    ) {
        reply.error(EROFS);
    }

    fn setattr(
        &mut self,
        _req: &Request,
        _ino: u64,
        _mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        _size: Option<u64>,
        _atime: Option<Timespec>,
        _mtime: Option<Timespec>,
        _fh: Option<u64>,
        _crtime: Option<Timespec>,
        _chgtime: Option<Timespec>,
        _bkuptime: Option<Timespec>,
        _flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        reply.error(EROFS);
    }
}


pub struct ReadWriteSeekFs<F: Read + Write + Seek>(ReadSeekFs<F>);

impl<F> ReadWriteSeekFs<F>
where
    F: Read + Write + Seek,
{
    pub fn new(mut f: F, bs: usize) -> Result<ReadWriteSeekFs<F>> {
        Ok(ReadWriteSeekFs(ReadSeekFs::new(f,bs)?))
    }

    fn seek_and_write(&mut self, offset: i64, data: &[u8]) -> Result<usize> {
        self.0.seek(offset)?;
        self.0.file.write_all2(data)
    }
}

impl<F> Filesystem for ReadWriteSeekFs<F>
where
    F: Read + Write + Seek,
{
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        reply.entry(&TTL, &self.0.fa, 0);
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        reply.attr(&TTL, &self.0.fa);
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        reply: ReplyData,
    ) {
        self.0.read(_req, ino, _fh, offset, size, reply)
    }

    
    fn write(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _flags: u32,
        reply: ReplyWrite,
    ) {
        match self.seek_and_write(offset, data) {
            Ok(len) => reply.written(len as u32),
            Err(e) => reply.error(errmap(e)),
        }
    }

    fn flush(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _lock_owner: u64,
        reply: ReplyEmpty,
    ) {
        match self.0.file.flush() {
            Ok(()) => reply.ok(),
            Err(e) => reply.error(errmap(e)),
        }
    }

    fn setattr(
        &mut self,
        _req: &Request,
        _ino: u64,
        _mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        _size: Option<u64>,
        _atime: Option<Timespec>,
        _mtime: Option<Timespec>,
        _fh: Option<u64>,
        _crtime: Option<Timespec>,
        _chgtime: Option<Timespec>,
        _bkuptime: Option<Timespec>,
        _flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        reply.attr(&TTL, &self.0.fa);
    }
}
