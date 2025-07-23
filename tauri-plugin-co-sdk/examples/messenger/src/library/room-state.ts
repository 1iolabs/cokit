import { CID } from "multiformats";
import { resolveCid, Room, sessionClose, sessionOpen } from "../../../../dist-js/index.js";

export async function getRoomState(
  co: string,
  coreId: string,
  stateCid: CID,
  externalSessionId?: string,
): Promise<Room.Room | undefined> {
  const sessionId = externalSessionId ?? (await sessionOpen(co));
  const coState = await resolveCid(sessionId, stateCid);
  const core = coState.cores[coreId];
  if (core) {
    const result = await resolveCid(sessionId, core.state);
    if (externalSessionId === undefined) {
      // close session if not given from external
      await sessionClose(sessionId);
    }
    return result;
  }
  if (externalSessionId === undefined) {
    // close session if not given from external
    await sessionClose(sessionId);
  }
  return undefined;
}
