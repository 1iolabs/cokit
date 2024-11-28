import { isPluginInitializeAction } from "@1io/kui-application-sdk";
import { Action } from "redux";
import { filter, identity, mergeAll, mergeMap, observeOn, queueScheduler, withLatestFrom } from "rxjs";
import { get_actions } from "../../../../../../dist-js/index.js";
import { createCoSdkStateEventListener } from "../../../library/co-sdk-state-listener.js";
import { getRoomState } from "../../../library/room-state.js";
import { MessengerViewActionType, MessengerViewNameChangedAction } from "../actions/index.js";
import { handleMatrixEvent } from "../library/handle-matrix-event.js";
import { MessengerViewEpicType } from "../types/plugin.js";

export const subscribeTauriEventEpic: MessengerViewEpicType = (action$, state$, context) => action$.pipe(
    filter(isPluginInitializeAction),
    withLatestFrom(state$),
    mergeMap(([, state]) => {
        return createCoSdkStateEventListener().pipe(
            observeOn(queueScheduler),
            withLatestFrom(state$),
            // only take events of this co
            filter(([event, state]) => { const [co] = event.payload; return co === state.co }),
            mergeMap(async ([event, state]) => {
                const [co, stateCid, heads] = event.payload;
                const actions: Action[] = [];
                if (stateCid) {
                    const roomState = await getRoomState(state.co, state.core, stateCid);
                    console.log("core", roomState);
                    if (roomState && roomState.name !== state.chatName) {
                        actions.push(identity<MessengerViewNameChangedAction>({
                            payload: { newName: roomState.name },
                            type: MessengerViewActionType.NameChanged,
                        }));
                    }

                }

                const latestMessage = state.messages.length > 0 ? state.messages[state.messages.length - 1] : undefined;

                // TODO: if there are over 200 new messages we might still need paging
                const log = (await get_actions(co, heads, 200, latestMessage?.actionCid)).actions;

                for (const cid of log) {
                    const roomChangeAction = await handleMatrixEvent(co, state.core, cid);
                    if (roomChangeAction !== undefined) {
                        actions.push(roomChangeAction);
                    }
                }

                // chronologically earliest action comes first from the get_actions() call,
                // but we save it in reverse order so we can just push the action when a new one comes in
                return actions.reverse();
            }),
        );
    }),
    mergeAll(),
);
