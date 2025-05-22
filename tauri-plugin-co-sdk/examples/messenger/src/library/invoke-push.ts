import moment from "moment";
import { CID } from "multiformats";
import * as json from 'multiformats/codecs/json';
import { sha256 } from 'multiformats/hashes/sha2';
import * as uuid from "uuid";
import { Messaging, pushAction, sessionClose, sessionOpen } from "../../../../dist-js/index.js";

export async function createCid<T>(data: T) {
    let json_data = json.encode(data);
    let hash = await sha256.digest(json_data);
    let cid = CID.createV1(0x55, hash);
    return cid;
}

export async function invokePushMessage(message: string, co: string, core: string, identity: string) {
    const session = await sessionOpen(co);
    let action: Messaging.MatrixEvent = {
        event_id: uuid.v4(),
        timestamp: moment.now(),
        room_id: "@some.room",
        type: "m_room_message",
        content: {
            msgtype: "text",
            body: message,
        }
    };
    await pushAction(session, core, action, identity);
    await sessionClose(session);
}
