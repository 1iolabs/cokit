import { Message } from "@1io/coapp-messenger-view";
import { Messaging } from "../../../../dist-js/index.js";

export interface ReadReceipt {
  messageId: string;
}

export function isMessage(v: any): v is Message {
  const message = v as Message;
  return message.message !== undefined && message.key !== undefined && message.timestamp !== undefined;
}

/**
 * Takes data in ipld format and converts it into a message
 *
 * @param ipld Data that has been fetched from a CO using the resolveCid function
 * @returns The converted message message
 */
export function resolveMatrixAction(ipld: any, ownIdentity?: string): Message | ReadReceipt | undefined {
  if (ownIdentity === undefined) {
    return undefined;
  }
  // handle matrix event
  const matrixEvent = ipld.p as Messaging.MatrixEvent;
  switch (matrixEvent.type) {
    case "m_room_message": {
      // handle new message
      return {
        key: matrixEvent.event_id,
        message: matrixEvent.content.body,
        ownMessage: ipld.f === ownIdentity,
        timestamp: new Date(matrixEvent.timestamp),
      };
    }
    case "m_receipt": {
      return { messageId: matrixEvent.content.m_read };
    }
  }
  return undefined;
}
