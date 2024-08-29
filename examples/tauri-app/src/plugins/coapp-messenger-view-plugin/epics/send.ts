import { EMPTY, filter, mergeAll, mergeMap } from "rxjs";
import { invokePushMessage } from "../../../library/invoke-push.js";
import { MessengerViewActionType, MessengerViewSendAction } from "../actions/index.js";
import { MessengerViewEpicType } from "../types/plugin.js";

export const sendEpic: MessengerViewEpicType = (action$, state$, context) => action$.pipe(
    filter((action): action is MessengerViewSendAction => action.type === MessengerViewActionType.Send),
    mergeMap(async (action) => {
        await invokePushMessage(action.payload.message, "1io");
        return EMPTY;
    }),
    mergeAll(),
);