import moment from "moment";
import { CID } from "multiformats";
import { sha256 } from "multiformats/hashes/sha2";
import * as uuid from "uuid";
import { Messaging, pushAction, sessionClose, sessionOpen } from "../../../../dist-js/index.js";
import { encode } from "@ipld/dag-cbor";
import { buildCoCoreId } from "./core-id.js";
import { Message } from "@1io/coapp-messenger-view";

export async function createCid<T>(data: T): Promise<CID> {
  const cborData = encode(data);
  const hash = await sha256.digest(cborData);
  const cid = CID.createV1(0x71, hash);
  return cid;
}

export async function invokePushReadReceipt(
  co: string,
  core: string,
  identity: string,
  messageId: string,
  externalSession?: string,
) {
  const session = externalSession ?? (await sessionOpen(co));
  const action: Messaging.MatrixEvent = {
    type: "m_receipt",
    event_id: uuid.v4(),
    room_id: buildCoCoreId(co, core),
    timestamp: moment.now(),
    content: { m_read: messageId },
  };
  try {
    await pushAction(session, core, action, identity);
  } catch (e) {
    console.error(e);
  }
  if (externalSession === undefined) {
    sessionClose(session);
  }
}

export function invokePushMessage(
  message: string, co: string, core: string, identity: string, session: string,
): Message {
  const action: Messaging.MatrixEvent = {
    content: {
      body: message,
      msgtype: "text",
    },
    event_id: uuid.v4(),
    room_id: buildCoCoreId(co, core),
    timestamp: moment.now(),
    type: "m_room_message",
  };
  pushAction(session, core, action, identity);
  return {
    key: action.event_id,
    message: action.content.body,
    ownMessage: true,
    timestamp: new Date(action.timestamp),
  };
}
