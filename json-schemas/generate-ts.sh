#!/bin/bash
set -e

# change directory to where this script is located to ensure script does not depend on where it's called from
DIR="$( cd "$( dirname "$0" )" && pwd )"
cd $DIR

echo "==== Generating types script start ===="

# clean folder
echo "Cleanup schemas and types folders"
rm -rf ./schemas/*
rm -rf ./types/*

# move to parent folder
cd ..

# run co-cli command to generate json schemas for room core, messaging and cores
echo "Calling co cli schema generate command"
cargo run --bin co schemars generate -m room messaging cores key

# generate .d.ts files, needs globally installed 'json-schema-to-typescript' npm package
echo "Calling json2ts command"
cd $DIR
npx "--package=json-schema-to-typescript@15.0.4" -- json2ts -i schemas -o types --no-additionalProperties

# change all .d.ts files to .ts files
echo "Renaming .d.ts files to .ts"
cd ./types
for file in ./*.d.ts;
do
    mv "$file" "${file%.d.ts}.ts"
done

cd ../..
# clean tauri types folder
echo "Cleanup tauri folders"
rm -rf ./tauri-plugin-co-sdk/guest-js/types/generated

# copy all generated types to tauri plugin
echo "Copy generated files into tauri guest-js folder"
cp -R json-schemas/types/ tauri-plugin-co-sdk/guest-js/types/generated

# build guest js
echo "Building tauri plugin packages"
cd tauri-plugin-co-sdk
npm run build

echo "DONE"
