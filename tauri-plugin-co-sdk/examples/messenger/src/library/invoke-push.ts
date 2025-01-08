import moment from "moment";
import { CID } from "multiformats";
import * as json from 'multiformats/codecs/json';
import { sha256 } from 'multiformats/hashes/sha2';
import * as uuid from "uuid";
import { pushAction } from "../../../../dist-js/index.js";
import { MatrixEvent } from "../types/matrix-event.js";

export async function createCid<T>(data: T) {
    let json_data = json.encode(data);
    let hash = await sha256.digest(json_data);
    let cid = CID.createV1(0x55, hash);
    return cid;
}

export async function invokePushMessage(message: string, co: string, core: string, identity: string) {
    let action: MatrixEvent = {
        event_id: uuid.v4(),
        timestamp: moment.now(),
        room_id: "@some.room",
        type: "m_room_message",
        content: {
            msgtype: "text",
            body: message,
        }
    };
    await pushAction(co, core, action, identity);
}
