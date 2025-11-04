import { CoMap, BlockStorage } from "./pkg";

let blocks = new Map();
let storage = new BlockStorage(
  async (cid) => blocks.get(cid),
  async (cid, data) => {
    blocks.set(cid, data);
  },
);

let map = new CoMap();
console.log("co", map);
await map.insert(storage, "hello", "world");
console.log("blocks", blocks);
