#!/usr/bin/env bash

#change directory to parent folder of where the script is located
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
cd $SCRIPT_DIR/..
typeshare co-messaging/src/lib.rs -l typescript -d ./types/typescript
