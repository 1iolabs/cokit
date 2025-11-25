import { CID } from "multiformats";
import { CoMap, BlockStorage, unixfsAdd } from "./pkg";
import "web-streams-polyfill";

function newStorage() {
  let blocks = new Map();
  let storage = new BlockStorage(
    async (cid) => {
      console.log("Getting block ", blocks, cid);
      const block = blocks.get(CID.decode(cid).toString());
      // const block = blocks.get(cid);
      console.log("Got block: ", block);
      return block;
    },
    async (cid_bytes, data) => {
      const cid = CID.decode(cid_bytes);
      console.log("Setting block ", cid.toString(), data);
      blocks.set(cid.toString(), data);
      return cid.bytes;
    },
  );
  return [storage, blocks];
}

async function test_stream() {
  console.log("Test stream:\n");
  const [storage, blocks] = newStorage();
  let map = new CoMap();
  console.log("co", map, blocks);
  await map.insert(storage, "hello", "world");
  console.log("co 2", map, blocks);
  const stream = map.stream(storage);
  for await (const i of stream) {
    console.log("stream content", i);
  }
}

async function test_unixfs_add_empty() {
  console.log("Test unixfs add:\n");
  const [storage, blocks] = newStorage();
  console.log("test add empty", storage, blocks);
  const stream = new ReadableStream([]);
  const cids = await unixfsAdd(storage, stream);
  console.log("test cid count: ", cids.length === 1);

  console.log("testing cid: ", cids[0] === "QmbFMke1KXqnYyBBWxB74N4c5SBnJMVAiMNRcGu6x1AwQH");
}

async function tests() {
  // await test_stream();
  await test_unixfs_add_empty();
}

tests();
