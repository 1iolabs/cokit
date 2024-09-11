import { encode } from "@ipld/dag-cbor";
import { invoke } from "@tauri-apps/api/core";
import moment from "moment";
import { CID } from "multiformats";
import * as json from 'multiformats/codecs/json';
import { sha256 } from 'multiformats/hashes/sha2';
import * as uuid from "uuid";

export async function createCid<T>(data: T) {
    let json_data = json.encode(data);
    let hash = await sha256.digest(json_data);
    let cid = CID.createV1(0x55, hash);
    return cid;
}

export async function invokePushMessage(message: string, co: string, core: string) {
    let action = {
        event_id: uuid.v4(),
        timestamp: moment.now(),
        room_id: "@some.room",
        type: "m.room.message",
        content: {
            msgtype: "m.text",
            body: message,
        }
    };
    await invokePush(action, co, core);
}

export async function invokePush(action: object, co: string, core: string) {
    let body_raw = encode({ action, co, core });
    await invoke("push", { body: Array.from(body_raw) });
}
