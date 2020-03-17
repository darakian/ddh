### Linux Build Status
[![Build Status](https://travis-ci.org/darakian/ddh.svg?branch=master)](https://travis-ci.org/darakian/ddh)
### Windows Build Status
[![Build status](https://ci.appveyor.com/api/projects/status/wv7tcfn0a7grjnv6?svg=true)](https://ci.appveyor.com/project/darakian/ddh)

# The **D**irectory **D**ifferential **h**Tool
DDH traverses input directories and their subdirectories. It also hashes files as needed and reports findings.

The H in "hTool" is silent. The H in its abbreviation, "DDH," is not.

This tool is called DDH for two very good reasons.
* DDT is a dangerous pesticide
* I mistyped when I created the project

## Usage
DDH is usable both as a library and as a stand alone CLI tool and aims to be simple to use in both cases.

## Library example
```rust
let (files, errors): (Vec<Fileinfo>, Vec<(_, _)>) = ddh::deduplicate_dirs(dirs);
let (shared, unique): (Vec<&Fileinfo>, Vec<&Fileinfo>) = files
                    .par_iter()
                    .partition(|&x| x.get_paths().len()>1);
process_full_output(&shared, &unique, &files, &errors, &arguments);
```

## CLI Install
* Install [Rust](https://www.rust-lang.org/en-US/install.html)
* `cargo install --git https://github.com/darakian/ddh ddh`
* The DDH binary will be installed into `$CARGO_HOME/.bin/ddh`, which usually is `$HOME/.cargo/bin/ddh`. This should be in your `PATH` already if you're using rustup.

## CLI Features
DDH supports both a `standard` output for human comprehension and a parsable `json` output for custom tools such as [ddh-move](https://github.com/JayWalker512/ddh-move).

## CLI Example
```
Directory Difference hTool
Jon Moroney jmoroney@hawaii.edu
Compare and contrast directories.
Example invocation: ddh /home/jon/downloads /home/jon/documents -f duplicates
Example pipe: ddh ~/Downloads/ -o no -v all -f json | someJsonParser.bin

USAGE:
    ddh [OPTIONS] <Directories>...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -b, --blocksize <Blocksize>    Sets the display blocksize to Bytes, Kilobytes, Megabytes or Gigabytes. Default is
                                   Kilobytes. [possible values: B, K, M, G]
    -f, --format <Format>          Sets output format. [possible values: standard, json, off]
    -o, --output <Output>          Sets file to save all output. Use 'no' for no file output.
    -v, --verbosity <Verbosity>    Sets verbosity for printed output. [possible values: quiet, duplicates, all]

ARGS:
    <Directories>...    Directories to parse
```
## How Does DDH Work?
DDH works by hashing files to determine their uniqueness and, as such, depends heavily on disk speeds for performance. The algorithmic choices in use are discussed [here](https://darakian.github.io/2018/04/02/how-many-bytes-does-it-take.html).
