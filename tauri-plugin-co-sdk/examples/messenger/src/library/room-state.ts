import { CID } from "multiformats";
import { Room } from "../types/room.js";
import { invokeResolveCid } from "./invoke-get.js";

export async function getRoomState(co: string, coreId: string, stateCid: CID): Promise<Room | undefined> {

    const coState = await invokeResolveCid(co, stateCid);
    const core = coState.cores[coreId];
    if (core) {
        return await invokeResolveCid(co, core.state);
    }
    return undefined;
}
