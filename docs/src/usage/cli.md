# CO CLI
The built-in co CLI helps to inspect and interact with COs.

## Commands

The CLI is organized into command and sub-commands.

The `help` command (or `-h`, `--help`) can be used to get a description of the command and its arguments.

Below, some useful commands are outlined.

### `co co ls`

List all COs.

### `co network listen`

Listen for connections using peer-to-peer networking.

### `co core build`

Expects the current directory to be a rust core crate and attempts to build it to [WebAssembly](../glossary/glossary.md#wasm).

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
