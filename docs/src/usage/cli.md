# CO CLI
#todo
The builtin co CLI helps to inspect and interact with COs. 

## Usage

```
% co help
CO CLI

Usage: co [OPTIONS] <COMMAND>

Commands:
  co        CO
  network   Network Utilities
  core      COre related commands
  ipld      IPLD Utilities
  did       Identities
  storage   Block Storage
  file      File
  room      Room
  pin       Pin
  schemars  Json schemas
  help      Print this message or the help of the given subcommand(s)

Options:
      --instance-id <INSTANCE_ID>
          The instance ID of the daemon. Must be uniqure for every instance that runs in parallel [default: co-cli]
      --base-path <BASE_PATH>
          Base path
      --log-path <LOG_PATH>
          Log path
      --no-log
          Disable logging to file
      --log-level <LOG_LEVEL>
          Only log level and above [default: info] [possible values: error, warn, info, debug, trace]
      --no-keychain
          Read/Write Local CO encryption key to file instead of the OS keychain
  -q
          No output
  -v...
          Verbose level. By default prints info and above levels. To prevent this use `quiet` option
      --open-telemetry
          Enable open telemetry tracing to endpoint
      --open-telemetry-endpoint <OPEN_TELEMETRY_ENDPOINT>
          Open telemetry endpoint [default: http://localhost:4317]
      --no-default-features
          Disable default features
  -F, --feature <FEATURE>
          Enable feature
  -h, --help
          Print help (see more with '--help')
```

```
% co help co
CO

Usage: co co <COMMAND>

Commands:
  ls      List all local COs
  cat     Print a block
  create  Create a new CO
  remove  Remove/Leave a CO
  log     Print CO Log
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

```
% co help network
Network Utilities

Usage: co network [OPTIONS] <COMMAND>

Commands:
  listen  Listen for connections
  help    Print this message or the help of the given subcommand(s)

Options:
      --force-new-peer-id  Force to create a new PeerId
  -h, --help               Print help
```

```
% co help core
COre related commands

Usage: co core <COMMAND>

Commands:
  build          Build COre binary
  build-builtin  Build built-on COre binaries
  inspect        Inspect COre binary
  help           Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

```
% co help ipld
IPLD Utilities

Usage: co ipld <COMMAND>

Commands:
  print-cbor   Print cbor from file
  inspect-cid  Inspect CID
  help         Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

```
% co help did 
Identities

Usage: co did <COMMAND>

Commands:
  ls      List identities
  invite  Invite participant to an CO
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

```
% co help storage
Block Storage

Usage: co storage <COMMAND>

Commands:
  cat   Print a block
  gc    Free unreferenced blocks
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

```
% co help file   
File

Usage: co file [OPTIONS] <CO> <COMMAND>

Commands:
  ls     List directory contents
  mkdir  Create directory
  cat    Print file contents
  add    Add new file
  rm     Remove file
  help   Print this message or the help of the given subcommand(s)

Arguments:
  <CO>  The CO ID

Options:
      --core <CORE>  The File Core Name [default: file]
  -h, --help         Print help
```

```
% co help room
Room

Usage: co room [OPTIONS] <CO> <COMMAND>

Commands:
  create  
  send    
  get     
  edit    
  help    Print this message or the help of the given subcommand(s)

Arguments:
  <CO>  ID of the co

Options:
      --core <CORE>  The room core name [default: room]
  -h, --help         Print help
```

```
% co help schemars
Json schemas

Usage: co schemars <COMMAND>

Commands:
  generate  Used to generate Json schemas of specified modules
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

