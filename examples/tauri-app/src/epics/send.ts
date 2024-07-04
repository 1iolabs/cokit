import { EMPTY, filter, mergeAll, mergeMap } from "rxjs";
import { MessengerActionType, MessengerSendAction } from "../actions";
import { MessengerEpicType } from "../types/plugin";
import { invoke_push_message } from "../library/invoke-push";

export const sendEpic: MessengerEpicType = (action$, state$, context) => action$.pipe(
    filter((action): action is MessengerSendAction => action.type === MessengerActionType.Send),
    mergeMap(async (action) => {
        await invoke_push_message(action.payload.message, "1io");
        return EMPTY;
    }),
    mergeAll(),
);