import { EMPTY, filter, mergeAll, mergeMap } from "rxjs";
import { MessengerViewActionType, MessengerViewSendAction } from "../actions/index.js";
import { MessengerViewEpicType } from "../types/plugin.js";
import { invoke_push_message } from "../../../library/invoke-push.js";

export const sendEpic: MessengerViewEpicType = (action$, state$, context) => action$.pipe(
    filter((action): action is MessengerViewSendAction => action.type === MessengerViewActionType.Send),
    mergeMap(async (action) => {
        await invoke_push_message(action.payload.message, "1io");
        return EMPTY;
    }),
    mergeAll(),
);