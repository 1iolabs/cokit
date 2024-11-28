import { CID } from "multiformats";
import { identity } from "rxjs";
import { invokeResolveCid } from "../../../library/invoke-get.js";
import { MatrixEvent } from "../../../types/matrix-event.js";
import { MessengerViewActions, MessengerViewActionType, MessengerViewAddMessageAction } from "../actions/index.js";

/**
 * Takes room core actions and converts them into an action that can be dispatched in this plugins context if possible
 * 
 * @param co Id of the co in which the action got triggered
 * @param core Id of the core for which you want handle actions
 * (Action may be of a different core. This filters out those situations)
 * @param actionCid Cid of the core reducer action
 * @param appendTop Adds prop to returned action payloads to append the received message to the top instead of bottom
 * @returns An action that can be dispatched in this plugin or undefined if the event couldn't be handled or was filtered
 */
export async function handleMatrixEvent(
    co: string, core: string, actionCid: CID, appendTop?: boolean,
): Promise<MessengerViewActions | undefined> {
    // resolve cid for reducer action
    const payload = await invokeResolveCid(co, actionCid);

    // make sure action is of given core
    if (payload.c !== core) {
        return undefined;
    }

    // handle matrix event
    const matrixEvent = payload.p as MatrixEvent;
    switch (matrixEvent.type) {
        case "m_room_message": {
            // handle new message
            return identity<MessengerViewAddMessageAction>({
                payload: {
                    message: { key: actionCid.toString(), message: matrixEvent.content.body, actionCid, ownMessage: true, timestamp: new Date(matrixEvent.timestamp) },
                    appendTop,
                },
                type: MessengerViewActionType.MessageReceived,
            });
            break;
        };
    }
    return undefined;
}
