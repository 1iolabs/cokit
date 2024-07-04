import { encode } from "@ipld/dag-cbor";
import { invoke } from "@tauri-apps/api/core";
import moment from "moment";
import { CID } from "multiformats";
import * as json from 'multiformats/codecs/json'
import { sha256 } from 'multiformats/hashes/sha2'
import * as uuid from "uuid";

export async function create_cid<T>(data: T) {
    let json_data = json.encode(data);
    let hash = await sha256.digest(json_data);
    let cid = CID.createV1(0x55, hash);
    return cid;
}

export async function invoke_push_message(message: string, co: string) {
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
    await invoke_push(action, co, "room");
}

export async function invoke_push(action: object, co: string, core: string) {
    let body_raw = encode({ action, co, core });
    await invoke("push", { body: Array.from(body_raw) });
}
