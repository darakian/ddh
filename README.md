# The **D**irectory **D**ifferential **h**Tool
The H is silent. This tool is called DDH for two very good reasons
* DDT is a dangerous pesticide
* I mistyped when I created the project

DDH takes two directories as arguments and returns the union or disunion (depending on an option third argument).

## Example
```
Directory Difference hTool 0.9.1
Compare and contrast directories.
Example invocation: ddh /home/jon/downloads /home/jon/documents -p S

USAGE:
    ddh [OPTIONS] <Directories>...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -b, --blocksize <Blocksize>    Sets the display blocksize to Bytes, Kilobytes, Megabytes or Gigabytes. Default is
                                   Kilobytes. [values: B, K, M, G]
    -p, --print <Print>            Print Single Instance or Shared Instance files. [values: single, shared, csv]

ARGS:
    <Directories>...    Directories to parse
````
