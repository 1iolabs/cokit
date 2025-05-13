# change directory to where this script is located to ensure script does not depend on where it's called from
DIR="$( cd "$( dirname "$0" )" && pwd )"
cd $DIR

# clean folder
rm -rf ./schemas/*
rm -rf ./types/*

# move to parent folder
cd ..

# run co-cli command to generate json schemas for room core, messaging and cores
cargo run --bin co-cli -- --no-keychain schemars generate -m room messaging cores

# generate .d.ts files, needs globally installed 'json-schema-to-typescript' npm package
cd $DIR
json2ts -i schemas -o types --no-additionalProperties

# change all .d.ts files to .ts files
cd ./types
for file in ./*.d.ts;
do 
    mv "$file" "${file%.d.ts}.ts"
done

cd ../..
# clean tauri types folder
rm -rf ./tauri-plugin-co-sdk/guest-js/types
# copy all generated types to tauri plugin
cp -R json-schemas/types tauri-plugin-co-sdk/guest-js

# build guest js
cd tauri-plugin-co-sdk
npm run build
