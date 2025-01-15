# change directory to where this script is located to ensure script does not depend on where it's called from
DIR="$( cd "$( dirname "$0" )" && pwd )"
cd $DIR

# copy files from types json-schemas/folder
cp ../../../../json-schemas/types/matrix-event.d.ts ../src/types
cp ../../../../json-schemas/types/room.d.ts ../src/types
