import { CID } from "multiformats";
import { CoMap, BlockStorage } from "./pkg";

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

let map = new CoMap();
console.log("co", map);
await map.insert(storage, "hello", "world");
console.log("blocks", blocks);
const stream = map.stream(storage);
for await (const i of stream) {
  console.log("stream content", i);
}
