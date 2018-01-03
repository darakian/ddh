# The **D**irectory **D**ifferential **h**Tool
The H is silent. This tool is called DDH for two very good reasons
* DDT is a dangerous pesticide
* I mistyped when I created the project

DDH takes two directories as arguments and returns the union or disunion (depending on an option third argument).

## Example
```
Directory Difference hTool 0.4.0
Compare and contrast directories

USAGE:
    ddh [FLAGS] [OPTIONS] <Directories>...

FLAGS:
    -h, --hidden     Searches hidden folders. NOT YET IMPLEMENTED. CURRENTLY TRUE.
        --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -b, --blocksize <Blocksize>    Sets the display blocksize to Kilobytes, Megabytes or Gigabytes. Default is Bytes.
                                   [values: K, M, G]
    -p, --print <Print>            Print Unique or Shared files. [values: U, S]

ARGS:
    <Directories>...    Directories to parse
````
