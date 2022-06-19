# QR Share

## Description

This tool allows sharing a file from a computer to a camera-capable device, assuming that both devices are on the same network (and that the network firewall is defined such that the camera-capable device can reach a certain TCP port on the computer).

## Installation

This tool is implemented in Rust and intended for Windows, macOS and Linux.  Assuming `cargo` is available on the PATH, one can install the tool (for the current user) via the following commands.

``` console
$ cargo install --release
```

If installing the tool is not the preferred approach, one can also run the tool directly via the following commands. (See below for further explanations on `qrshare` options.)

``` console
$ cargo run --release -- [qrshare options]
```

## Usage

### Synopsis

``` console
$ qrshare --help
USAGE:
    qrshare [OPTIONS] [FILES]...

ARGS:
    <FILES>...    The paths of files to serve.  There should be at least one file to serve

OPTIONS:
    -b, --bind <BIND>              Sets a custom bound address, default is all available addresses.
                                   UNIMPLEMENTED
    -h, --help                     Print help information
    -p, --port <PORT>              Sets a custom port.  Default to 0, where an arbitrary available
                                   port is used
        --png <PNG>                Use PNG format when generating the QR code.  This is the default.
                                   Conflicts with `--svg`.  Ignored when `--no-qrcode` is set
                                   [possible values: true, false]
    -q, --quiet <QUIET>            Quiet operation.  Do not warn about missing files [possible
                                   values: true, false]
    -Q, --no-qrcode <NO_QRCODE>    Do not show the QR code.  Overrides `--svg` and `--png` [possible
                                   values: true, false]
    -s, --strict <STRICT>          Strict mode.  When enabled, the server exits on any failure in
                                   path resolution and IO [possible values: true, false]
        --svg <SVG>                Use SVG format when generating the QR code.  Conflicts with
                                   `--png`, and is ignored when `--no-qrcode` is set [possible
                                   values: true, false]
    -V, --version                  Print version information
```

### Examples

``` console
$ qrshare --port 10000 file1 file2 file3
```

### Limitations

1. Currently the tool is only intended for static regular files -- that is, files like FIFO and Unix socket files are not taken into consideration when implementing the tool, and may not behave properly, and files that change frequently may also not be served properly.
2. Currently the URL for the _first file only_ will be encoded to QR code and displayed, while the remaining files do not have an easy way to be accessed by recipients.
   - Potentially the tool can be run as a daemon, where a controller program instructs it to add new files, and/or show new QR codes on screen.
