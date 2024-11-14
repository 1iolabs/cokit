import { isPluginInitializeAction } from "@1io/kui-application-sdk";
import { Action } from "redux";
import { filter, identity, mergeAll, mergeMap, observeOn, queueScheduler, withLatestFrom } from "rxjs";
import { get_actions } from "../../../../../../dist-js/index.js";
import { createCoSdkStateEventListener } from "../../../library/co-sdk-state-listener.js";
import { invokeResolveCid } from "../../../library/invoke-get.js";
import { MatrixEvent } from "../../../types/types.js";
import { MessengerViewActionType, MessengerViewNameChangedAction, MessengerViewReceivedAction } from "../actions/index.js";
import { MessengerViewEpicType } from "../types/plugin.js";

export const subscribeTauriEventEpic: MessengerViewEpicType = (action$, state$, context) => action$.pipe(
    filter(isPluginInitializeAction),
    withLatestFrom(state$),
    mergeMap(([, state]) => {
        return createCoSdkStateEventListener().pipe(
            filter((event) => { const [co] = event.payload; return co !== "local" }),
            observeOn(queueScheduler),
            withLatestFrom(state$),
            mergeMap(async ([event, state]) => {
                const [co, _, heads] = event.payload;

                const latestMessage = state.messages.length > 0 ? state.messages[state.messages.length - 1] : undefined;

                // TODO: if there are over 200 new messages we might still need paging
                const log = (await get_actions(co, heads, 200, latestMessage?.actionCid)).actions;
                const actions: Action[] = [];
                for (const cid of log) {
                    const payload = await invokeResolveCid(co, cid);
                    // make sure action is of current core
                    if (payload.c !== state.core) {
                        continue;
                    }
                    const matrixEvent = payload.p as MatrixEvent;
                    switch (matrixEvent.type) {
                        case "m_room_message": {
                            actions.push(identity<MessengerViewReceivedAction>({
                                payload: {
                                    message: { message: matrixEvent.content.body, actionCid: cid, ownMessage: true, timestamp: new Date(matrixEvent.timestamp) }
                                },
                                type: MessengerViewActionType.MessageReceived,
                            }));
                            break;
                        };
                        case "State": {
                            if (matrixEvent.content.type === "room_name") {
                                actions.push(identity<MessengerViewNameChangedAction>({
                                    payload: {
                                        newName: matrixEvent.content.content.name,
                                    },
                                    type: MessengerViewActionType.NameChanged,
                                }));
                            }

                        }
                    }
                }
                return actions.reverse();
            }),
        );
    }),
    mergeAll(),
);
