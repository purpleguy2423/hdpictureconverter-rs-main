# <img src="web/public/icon-512.png" alt="Low-resolution blue-and-white Polaroid-like photo of a landscape inscribed inside a sprocket" height=120 /> HD Picture Converter for CE (in Rust)

[![Web app](https://img.shields.io/static/v1?label=Web%20app&message=taricorp.gitlab.io/hdpictureconverter-rs&color=informational)](https://taricorp.gitlab.io/hdpictureconverter-rs/)
![CI status](https://img.shields.io/gitlab/pipeline-status/taricorp/hdpictureconverter-rs)

This is a (re)implementation of a tool to convert images to the format used by
[TheLastMillennial's HD Picture
Viewer](https://github.com/TheLastMillennial/HD-Picture-Viewer)
for TI-83+ and -84+ CE graphing calculators, allowing the images to be
displayed on a calculator in much higher quality than is natively supported by
the calculators.

The original tool motivating creation of this one was implemented as a C# GUI
application that using external programs to generate output files. This
implementation is designed to be usable as a web app so users don't need to
download any software, significantly lowering the barriers to use. It's built
in Rust because Rust is good, and because it's easy to compile to WebAssembly
to run in a browser.

## Usage

The simplest way to use this tool is as a web application, hosted from this
repository at https://taricorp.gitlab.io/hdpictureconverter-rs/. It requires
support for relatively modern web technologies in the browser (WebAssembly,
Web Workers and ES6 javascript syntax notably), but all of those are commonly
available since 2019.

It also implements a command-line tool; from the repository root use
[Cargo](https://doc.rust-lang.org/cargo/) to run it and show the built-in
usage information: `cargo run --bin cli -- --help`.

The core conversion functionality is implemented as a Rust library crate
(which the command-line interface and web app both make use of) which could
be used by other tools if desired as well, though doing so requires at least
some ability to program in Rust and is beyond the scope of this documentation.

## License

This project's source code is made available under the terms of the 2-clause BSD
license. See the included LICENSE file for full text.

The sprocket design used as part of the icon is official art for the Rust
project from https://github.com/rust-lang/rust-artwork, made available under the
Creative Commons Attribution (CC-BY) license by the Rust Foundation.
