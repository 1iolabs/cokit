import { EMPTY, filter, mergeAll, mergeMap, withLatestFrom } from "rxjs";
import { invokePushMessage } from "../../../library/invoke-push.js";
import { MessengerViewActionType, MessengerViewSendAction } from "../actions/index.js";
import { MessengerViewEpicType } from "../types/plugin.js";

export const sendEpic: MessengerViewEpicType = (action$, state$, context) => action$.pipe(
    filter((action): action is MessengerViewSendAction => action.type === MessengerViewActionType.Send),
    withLatestFrom(state$),
    mergeMap(async ([action, state]) => {
        const identity = state.chatsListState?.identity;
        if (!identity) {
            return EMPTY;
        }
        await invokePushMessage(action.payload.message, state.co, state.core, identity);
        return EMPTY;
    }),
    mergeAll(),
);
