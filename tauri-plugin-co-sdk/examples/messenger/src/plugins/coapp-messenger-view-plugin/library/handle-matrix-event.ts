import { Message } from "@1io/coapp-messenger-view";
import { Messaging } from "../../../../../../dist-js/index.js";

/**
 * Takes data in ipld format and converts it into a message
 *
 * @param ipld Data that has been fetched from a CO using the resolveCid function
 * @returns The converted message message
 */
export function resolveMatrixAction(ipld: any, ownIdentity: string): Message | undefined {
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
  }
  return undefined;
}
