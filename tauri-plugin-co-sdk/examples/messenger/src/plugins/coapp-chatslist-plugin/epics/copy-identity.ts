import { EMPTY, filter, mergeAll, mergeMap, withLatestFrom } from "rxjs";
import { ChatsListActionType } from "../actions/index.js";
import { ChatsListEpicType } from "../types/plugin.js";

export const copyIdentityEpic: ChatsListEpicType = (action$, state$) => action$.pipe(
    filter((action) => action.type === ChatsListActionType.CopyIdentity),
    withLatestFrom(state$),
    mergeMap(async ([, state]) => {
        if (state.identity === undefined) {
            // should really not happen
            console.error("Could not copy identity: Identity not set");
            return EMPTY;
        }
        try {
            await navigator.clipboard.writeText(state.identity);
        } catch (err) {
            console.error("Could not copy identity: ", err);
        }
        return EMPTY;
    }),
    mergeAll(),
);
