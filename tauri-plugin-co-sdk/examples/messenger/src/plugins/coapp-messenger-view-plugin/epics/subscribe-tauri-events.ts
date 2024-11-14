import { isPluginInitializeAction } from "@1io/kui-application-sdk";
import { Action } from "redux";
import { filter, mergeAll, mergeMap, observeOn, queueScheduler, withLatestFrom } from "rxjs";
import { get_actions } from "../../../../../../dist-js/index.js";
import { createCoSdkStateEventListener } from "../../../library/co-sdk-state-listener.js";
import { handleMatrixEvent } from "../library/handle-matrix-event.js";
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
