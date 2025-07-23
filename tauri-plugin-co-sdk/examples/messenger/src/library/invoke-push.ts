import moment from "moment";
import { CID } from "multiformats";
import * as json from "multiformats/codecs/json";
import { sha256 } from "multiformats/hashes/sha2";
import * as uuid from "uuid";
import { Messaging, pushAction, sessionClose, sessionOpen } from "../../../../dist-js/index.js";

export async function createCid<T>(data: T) {
  const jsonData = json.encode(data);
  const hash = await sha256.digest(jsonData);
  const cid = CID.createV1(0x55, hash);
  return cid;
}

export async function invokePushMessage(message: string, co: string, core: string, identity: string) {
  const session = await sessionOpen(co);
  const action: Messaging.MatrixEvent = {
    content: {
      body: message,
      msgtype: "text",
    },
    event_id: uuid.v4(),
    room_id: "@some.room",
    timestamp: moment.now(),
    type: "m_room_message",
  };
  await pushAction(session, core, action, identity);
  await sessionClose(session);
}
