import { EMPTY, filter, mergeMap, withLatestFrom } from "rxjs";
import { ChatsListActionType, ChatsListSetDialogAction } from "../actions/index.js";
import { ChatsListEpicType } from "../types/plugin.js";

export const loadDialogEpic: ChatsListEpicType = (actino$, state$, context) => actino$.pipe(
    filter((action): action is ChatsListSetDialogAction => action.type === ChatsListActionType.SetDialog),
    withLatestFrom(state$),
    mergeMap(([action, state]) => {
        if (action.payload.dialogPluginId === undefined && state.dialog !== undefined) {
            // unload dialog
            context.api.unloadPlugin(state.dialog);
        }
        return EMPTY;
    }),
);
