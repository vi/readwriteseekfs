extern crate fuse;
extern crate readwriteseekfs;

use std::fs::File;
use std::io::{Cursor, Result, Write};

fn main() -> Result<()> {
    let content = vec![0; 65536];
    let mut c = Cursor::new(content);
    c.write(b"Hello, world\n")?;
    let fs = readwriteseekfs::ReadWriteSeekFs::new(c, 1024)?;
    let _ = File::create(&"hello.txt");
    fuse::mount(fs, &"hello.txt", &[])
}
