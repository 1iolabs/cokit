import { CID } from "multiformats";
import { CoMap, BlockStorage, unixfsAdd } from "./pkg";

function newStorage() {
  let blocks = new Map();
  let storage = new BlockStorage(
    async (cid) => {
      console.log("Getting block ", blocks, cid);
      const block = blocks.get(CID.decode(cid).toString());
      console.log("Got block: ", block);
      return block;
    },
    async (cid, data) => {
      console.log("Setting block ", cid, data);
      blocks.set(CID.decode(cid).toString(), data);
    },
  );
  return [storage, blocks];
}

async function test_stream() {
  const [storage, blocks] = newStorage();
  let map = new CoMap();
  console.log("co", map, blocks);
  await map.insert(storage, "hello", "world");
  const stream = map.stream(storage);
  for await (const i of stream) {
    console.log("stream content", i);
  }
}

async function test_unixfs_add_empty() {
  const [storage, blocks] = newStorage();
  console.log("test add empty", storage, blocks);
  const stream = new ReadableStream([]);
  const cids = await unixfsAdd(storage, stream).catch((e) => console.error(e));
  console.log("test cid count: ", cids.length === 1);

  console.log("testing cid: ", cids[0] === "QmbFMke1KXqnYyBBWxB74N4c5SBnJMVAiMNRcGu6x1AwQH");
}

// test_stream();
test_unixfs_add_empty();
