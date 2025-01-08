import { EMPTY, filter, mergeAll, mergeMap, withLatestFrom } from "rxjs";
import { invokePushMessage } from "../../../library/invoke-push.js";
import { MessengerViewActionType, MessengerViewSendAction } from "../actions/index.js";
import { MessengerViewEpicType } from "../types/plugin.js";

export const sendEpic: MessengerViewEpicType = (action$, state$, context) => action$.pipe(
    filter((action): action is MessengerViewSendAction => action.type === MessengerViewActionType.Send),
    withLatestFrom(state$),
    mergeMap(async ([action, state]) => {
        await invokePushMessage(action.payload.message, state.co, state.core, "did:local:device"); // TODO
        return EMPTY;
    }),
    mergeAll(),
);
