# change directory to where this script is located to ensure script does not depend on where it's called from
DIR="$( cd "$( dirname "$0" )" && pwd )"
cd $DIR

# move to parent folder
cd ..
# run co-cli command to generate json schemas for room core and messaging
cargo run --bin co-cli -- --no-keychain schemars generate -m room messaging

# generate .d.ts files, needs globally installed 'json-schema-to-typescript' npm package
cd $DIR
json2ts -i schemas -o types --additionalProperties false
