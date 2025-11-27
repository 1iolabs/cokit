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
  const [storage, blocks] = newStorage();
  let map = new CoMap();
  await map.insert(storage, "hello", "world");
  assertEq(blocks.size, 1);
  assertEq(
    CID.decode(map.cid()).toString(),
    "bafyr4ib4sqmbfbyhkoh64ylvnwrm3uyqhq43zeknhnfj643kpghqjdopza",
  );
  const stream = map.stream(storage);
  let values = [];
  for await (const i of stream) {
    values.push(i);
  }
  console.log("values: ", values);
  assertEq(values.length, 1);
  assertEq(values[0][0], "hello");
  assertEq(values[0][1], "world");
}

async function test_unixfs_add() {
  const [storage, blocks] = newStorage();
  var count = 64 * 1024;
  const stream = new ReadableStream({
    start(controller) {
      for (var i = 1024; i--; i > 0) {
        controller.enqueue(
          new TextEncoder().encode("hello world test".repeat(64)),
        );
      }
      controller.close();
    },
  });
  const cids = await unixfsAdd(storage, stream);
  console.log("cids: ", cids);
  assertEq(cids.length, 5);
  assertEq(
    CID.decode(cids[0]).toString(),
    "QmPEvxGmvxzfMews81gF5NMvFNeFAdNmhtwzGPhkHhoyqy",
  );
  assertEq(
    CID.decode(cids[1]).toString(),
    "QmPEvxGmvxzfMews81gF5NMvFNeFAdNmhtwzGPhkHhoyqy",
  );
  assertEq(
    CID.decode(cids[2]).toString(),
    "QmPEvxGmvxzfMews81gF5NMvFNeFAdNmhtwzGPhkHhoyqy",
  );
  assertEq(
    CID.decode(cids[3]).toString(),
    "QmPEvxGmvxzfMews81gF5NMvFNeFAdNmhtwzGPhkHhoyqy",
  );
  assertEq(
    CID.decode(cids[4]).toString(),
    "QmVRRmYKvn8m3jQT8VHX1BCgrQLFvzsB26aKwLCyFRvYSv",
  );
}

async function test_unixfs_add_empty() {
  console.log("Test unixfs add:\n");
  const [storage, blocks] = newStorage();
  console.log("test add empty", storage, blocks);
  const stream = new ReadableStream({
    start(controller) {
      // controller.enqueue(new TextEncoder().encode("hello"));
      controller.close();
    },
  });
  const cids = await unixfsAdd(storage, stream);
  console.log("cids: ", cids);
  assertEq(cids.length, 1);
  assertEq(
    CID.decode(cids[0]).toString(),
    "QmbFMke1KXqnYyBBWxB74N4c5SBnJMVAiMNRcGu6x1AwQH",
  );
}

async function test_async(func) {
  console.info("🧪 test:", func.name);
  await func();
}

function assertEq(actual, expected) {
  if (actual !== expected) {
    console.error("❌ failed:", expected, "!==", actual);
  } else {
    console.info("✅ passed:", expected, "===", actual);
  }
}

async function tests() {
  await test_async(test_stream);
  await test_async(test_unixfs_add);
  await test_async(test_unixfs_add_empty);
}
tests();
