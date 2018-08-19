readwriteseekfs
---

A [`fuse::Filesystem`](https://docs.rs/fuse/0.3.1/fuse/trait.Filesystem.html) implementation based on any [`Read`](https://doc.rust-lang.org/stable/std/io/trait.Read.html)`+Write+Seek` implementation. Using this library you can easily expose file-like objects in Rust as a mountable single-file FUSE filesystem.
Read-only files can be crated from just Read+Seek, without Write.

Example
---

See [examples/simple.rs](examples/simple.rs) for the code.

## Running the example

First console:

    cargo run --example=simple

Second console:

```
$ hd hello.txt 
00000000  48 65 6c 6c 6f 2c 20 77  6f 72 6c 64 0a 00 00 00  |Hello, world....|
00000010  00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 00  |................|
*
00010000

$ echo qwe > hello.txt

$ hd hello.txt 
00000000  71 77 65 0a 6f 2c 20 77  6f 72 6c 64 0a 00 00 00  |qwe.o, world....|
00000010  00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 00  |................|
*
00010000

$ fusermount -u hello.txt
```

Notes
---

* File size is determined using seeking at startup and is not changeable without unmounting
