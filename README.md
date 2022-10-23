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

Example invocation: ddh -v duplicates -d /home/jon/downloads /home/jon/documents
Example pipe: ddh -o no -v all -f json -d ~/Downloads/ | someJsonParser.bin

Usage: ddh [OPTIONS]

Options:
  -m, --minimum [<MIN_SIZE>]
          Minimum file size in bytes to consider [default: 0]
  -b, --blocksize [<BLOCKSIZE>]
          Set the display blocksize to Bytes, Kilobytes, Megabytes or Gigabytes [default: K] [possible values: B, K, M, G]
  -v, --verbosity [<VERBOSITY>]
          Set verbosity for printed output [default: quiet] [possible values: quiet, duplicates, all]
  -o, --output [<OUTPUT>]
          Set file to save all output. Use 'no' for no file output [default: Results.txt]
  -f, --format [<FMT>]
          Set output format [default: standard] [possible values: standard, json]
  -i, --ignore <IGNORE_DIRS>
          Directories to ignore (comma separated list)
  -d, --directories <DIRECTORIES>...
          Directories to parse
  -h, --help
          Print help information (use `--help` for more detail)
  -V, --version
          Print version information
```
## How Does DDH Work?
DDH works by hashing files to determine their uniqueness and, as such, depends heavily on disk speeds for performance. The algorithmic choices in use are discussed [here](https://darakian.github.io/2018/04/02/how-many-bytes-does-it-take.html).
