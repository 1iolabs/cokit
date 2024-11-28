import { Action } from "redux";
import { filter, identity, mergeAll, mergeMap, queueScheduler, throttleTime, withLatestFrom } from "rxjs";
import { get_actions, GetActionsResponse, getCoState } from "../../../../../../dist-js/index.js";
import { MessengerViewActionType, MessengerViewLoadMoreEventsAction, MessengerViewSetLastHeadsAction } from "../actions/index.js";
import { handleMatrixEvent } from "../library/handle-matrix-event.js";
import { MessengerViewEpicType } from "../types/plugin.js";

export const loadMoreEventsEpic: MessengerViewEpicType = (action$, state$, context) => action$.pipe(
    filter((action): action is MessengerViewLoadMoreEventsAction => action.type === MessengerViewActionType.LoadMoreEvents),
    throttleTime(500, queueScheduler),
    withLatestFrom(state$),
    mergeMap(async ([action, state]) => {
        // take last heads or current if undefined
        let heads = state.lastHeads;
        if (heads === undefined) {
            [, heads] = await getCoState(state.co);
        }
        const actions: Action[] = [];
        while (actions.length < action.payload.count) {
            let log: GetActionsResponse = await get_actions(state.co, heads, action.payload.count, undefined);
            for (const cid of log.actions) {
                const roomChangeAction = await handleMatrixEvent(state.co, state.core, cid, true);
                if (roomChangeAction !== undefined) {
                    actions.push(roomChangeAction);
                }
            }
            heads = log.next_heads;
            if (heads.size === 0) {
                // wouldn't get no more actions and cause a loop
                console.log("AAHHHH", heads);
                break;
            }
        }
        // save where we stopped getting the log for next call
        actions.push(identity<MessengerViewSetLastHeadsAction>(
            { payload: { lastHeads: heads }, type: MessengerViewActionType.SetLastHeads }
        ));

        // chronologically earliest action comes first from the get_actions() call,
        // but we save it in reverse order so we can just push the action when a new one comes in
        return actions;
    }),
    mergeAll(),
);