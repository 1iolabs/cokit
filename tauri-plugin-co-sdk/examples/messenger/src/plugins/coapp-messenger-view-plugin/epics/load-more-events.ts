import { CID } from "multiformats";
import { EMPTY, filter, identity, mergeAll, mergeMap, observeOn, queueScheduler, withLatestFrom } from "rxjs";
import { get_actions, GetActionsResponse, getCoState, resolveCid } from "../../../../../../dist-js/index.js";
import { MessengerViewActionType, MessengerViewAddMessagesAction, MessengerViewLoadMoreEventsAction, MessengerViewSetLastHeadsAction } from "../actions/index.js";
import { MessengerViewEpicType } from "../types/plugin.js";

export const loadMoreEventsEpic: MessengerViewEpicType = (action$, state$, context) => action$.pipe(
    filter((action): action is MessengerViewLoadMoreEventsAction => action.type === MessengerViewActionType.LoadMoreEvents),
    withLatestFrom(state$.pipe(filter((s) => s?.coSessionId !== undefined))),
    observeOn(queueScheduler),
    mergeMap(async ([action, state]) => {
        if (state.coSessionId === undefined) {
            return EMPTY;
        }
        // take last heads or current if undefined
        let heads = state.lastHeads;
        if (heads === undefined) {
            [, heads] = await getCoState(state.co);
        }
        // load actions
        const [messages, nextHeads] = await loadActionsCapped(state.coSessionId, state.core, action.payload.count, heads);
        return [
            // save where we stopped getting the log for next call
            identity<MessengerViewSetLastHeadsAction>(
                { payload: { lastHeads: nextHeads }, type: MessengerViewActionType.SetLastHeads }
            ),
            // save actions to state in reverse order
            identity<MessengerViewAddMessagesAction>({
                payload: { messages: messages.reverse(), appendTop: true },
                type: MessengerViewActionType.AddMessages,
            }),
        ];
    }),
    mergeAll(),
);

async function loadActionsCapped(sessionId: string, core: string, count: number, heads: CID[]): Promise<[CID[], CID[]]> {
    let newHeads = heads;
    const messages: CID[] = [];
    // we want at least the specified number of messages.
    // Actions can be from different cores so we cannot compare from fetched actions
    while (messages.length < count) {
        // fetch actions
        let log: GetActionsResponse = await get_actions(sessionId, newHeads, count, undefined);
        // update heads
        newHeads = log.next_heads;
        if (log.actions.length === 0) {
            // break if there are no more actions to get
            break;
        }
        for (const cid of log.actions) {
            const payload = await resolveCid(sessionId, cid);
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
