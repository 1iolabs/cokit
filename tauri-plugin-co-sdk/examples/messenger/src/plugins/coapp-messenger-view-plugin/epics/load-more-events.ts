import { CID } from "multiformats";
import { filter, identity, mergeAll, mergeMap, queueScheduler, throttleTime, withLatestFrom } from "rxjs";
import { get_actions, GetActionsResponse, getCoState } from "../../../../../../dist-js/index.js";
import { invokeResolveCid } from "../../../library/invoke-get.js";
import { MessengerViewActionType, MessengerViewAddMessagesAction, MessengerViewLoadMoreEventsAction, MessengerViewSetLastHeadsAction } from "../actions/index.js";
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
        // load actions
        const [messages, nextHeads] = await loadActionsCapped(state.co, state.core, action.payload.count, heads);
        return [
            // save actions to state in reverse order
            identity<MessengerViewAddMessagesAction>({
                payload: { messages: messages.reverse(), appendTop: true },
                type: MessengerViewActionType.AddMessages,
            }),
            // save where we stopped getting the log for next call
            identity<MessengerViewSetLastHeadsAction>(
                { payload: { lastHeads: nextHeads }, type: MessengerViewActionType.SetLastHeads }
            ),
        ];
    }),
    mergeAll(),
);

async function loadActionsCapped(co: string, core: string, count: number, heads: CID[]): Promise<[CID[], CID[]]> {
    let newHeads = heads;
    const messages: CID[] = [];
    // we want at least the specified number of messages.
    // Actions can be from different cores so we cannot compare from fetched actions
    while (messages.length < count) {
        // fetch actions
        let log: GetActionsResponse = await get_actions(co, newHeads, count, undefined);
        // update heads
        newHeads = log.next_heads;
        if (log.actions.length === 0) {
            // break if there are no more actions to get
            break;
        }
        for (const cid of log.actions) {
            const payload = await invokeResolveCid(co, cid);
            // check if this is the core creation action
            if (payload.c === "co") {
                if (payload.p?.CoreCreate !== undefined) {
                    if (payload.p.CoreCreate.core === core) {
                        // this is the core create action
                        // there is no more actions after this for this core so we have no more heads
                        return [messages, []];
                    }
                }
            }

            // ensure current core
            if (payload.c !== core) { continue; }
            messages.push(cid);
        }
    }
    return [messages, newHeads];
}
