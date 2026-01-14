import { CID } from "multiformats";
import { CoMap, BlockStorage, unixfsAdd, CoSet, CoList } from "./pkg";
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

async function test_co_map() {
  const [storage, blocks] = newStorage();
  let map = new CoMap();
  // test empty
  assertEq(map.is_empty(), true);
  // test insert
  await map.insert(storage, "hello", "world");
  assertEq(blocks.size, 1);
  assertEq(
    CID.decode(map.cid()).toString(),
    "bafyr4ib4sqmbfbyhkoh64ylvnwrm3uyqhq43zeknhnfj643kpghqjdopza",
  );
  assertEq(map.is_empty(), false);
  // test stream
  const stream = map.stream(storage);
  let values = [];
  for await (const i of stream) {
    values.push(i);
  }
  console.log("values: ", values);
  assertEq(values.length, 1);
  assertEq(values[0][0], "hello");
  assertEq(values[0][1], "world");
  // test contains
  assertEq(await map.contains(storage, "not contained"), false);
  assertEq(await map.contains(storage, "hello"), true);
  // test get
  assertEq(await map.get(storage, "hello"), "world");
}

async function test_co_set() {
  const [storage, _] = newStorage();
  let set = new CoSet();
  // test empty
  assertEq(set.is_empty(), true);
  // test insert
  await set.insert(storage, "hello");
  await set.insert(storage, "world");
  assertEq(set.is_empty(), false);
  // test stream
  let values = [];
  for await (const item of set.stream(storage)) {
    values.push(item);
  }
  assertEq(values[0], "hello");
  assertEq(values[1], "world");
  // test contains
  assertEq(await set.contains(storage, "not contained"), false);
  assertEq(await set.contains(storage, "hello"), true);
  assertEq(await set.contains(storage, "world"), true);
  // test remove
  assertEq(await set.remove(storage, "hello"), true);
  assertEq(await set.remove(storage, "hello"), false);
  assertEq(await set.contains(storage, "hello"), false);
}

async function test_co_list() {
  const [storage, _] = newStorage();
  const list = new CoList();
  // test push
  await list.push(storage, "hello");
  await list.push(storage, "world");
  await list.push(storage, "test");
  // test stream
  let values = [];
  for await (const item of list.stream(storage)) {
    values.push(item);
  }
  assertEq(values[0], "hello");
  assertEq(values[1], "world");
  assertEq(values[2], "test");
  // test reverse stream
  values = [];
  for await (const item of list.reverse_stream(storage)) {
    values.push(item);
  }
  assertEq(values[2], "hello");
  assertEq(values[1], "world");
  assertEq(values[0], "test");
  // pop
  assertEq(await list.pop_front(storage), "hello");
  assertEq(await list.pop(storage), "test");
}

async function test_unixfs_add() {
  const [storage, _] = newStorage();
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
  const div = document.createElement("div");
  const equalSignCount = 50 - (func.name.length + 10) / 2;
  div.textContent = `${"=".repeat(equalSignCount)} Testing ${func.name} ${"=".repeat(equalSignCount)}`;
  document.getElementById("main").appendChild(div);
  await func();
}

function assertEq(actual, expected) {
  if (actual !== expected) {
    console.error("❌ Failed:", expected, "!==", actual);
    const div = document.createElement("div");
    div.textContent = `❌ Failed: Wanted: ${expected} but got ${actual}`;
    document.getElementById("main").appendChild(div);
  } else {
    console.info("✅ Passed:", expected, "===", actual);
    const div = document.createElement("div");
    div.textContent = `✅ Passed: Value: ${actual}`;
    document.getElementById("main").appendChild(div);
  }
}

async function tests() {
  await test_async(test_co_map);
  await test_async(test_co_set);
  await test_async(test_co_list);
  await test_async(test_unixfs_add);
  await test_async(test_unixfs_add_empty);
}
tests();
