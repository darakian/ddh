### Linux Build Status
[![Build Status](https://travis-ci.org/darakian/ddh.svg?branch=master)](https://travis-ci.org/darakian/ddh)
### Windows Build Status
[![Build status](https://ci.appveyor.com/api/projects/status/wv7tcfn0a7grjnv6?svg=true)](https://ci.appveyor.com/project/darakian/ddh)

# The **D**irectory **D**ifferential **h**Tool
The H is silent. This tool is called DDH for two very good reasons
* DDT is a dangerous pesticide
* I mistyped when I created the project

DDH traverses input directories and their subdirectories, hashes files as needed and reports findings.

## Install
* Install [Rust](https://www.rust-lang.org/en-US/install.html)
* ``` git clone https://github.com/darakian/ddh.git ddh ```
* ``` cd ddh ```
* ``` cargo build --release ```
* the ddh binary will then be at target/release/ddh

## Features
DDH supports outputting to human readable `standard` output and to machine parsable `json`.

## Example
```
Directory Difference hTool 0.9.8
Compare and contrast directories.
Example invocation: ddh ~/Downloads/ -o MyFiles.txt -f standard
Example pipe ./target/release/ddh ~/Downloads/ -o no -v all -f json | someJsonParser.bin

USAGE:
    ddh [OPTIONS] <Directories>...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -b, --blocksize <Blocksize>    Sets the display blocksize to Bytes, Kilobytes, Megabytes or Gigabytes. Default is
                                   Kilobytes. [possible values: B, K, M, G]
    -f, --format <Format>          Sets output format. [possible values: standard, json]
    -o, --output <Output>          Sets file to save all output.
    -v, --verbosity <Verbosity>    Sets verbosity for printed output. [possible values: quiet, duplicates, all]

ARGS:
    <Directories>...    Directories to parse
```
## How
DDH works by hashing files to determine uniqueness and as such depends heavily on disk speeds for performance.
