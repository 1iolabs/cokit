import { isPluginInitializeAction } from "@1io/kui-application-sdk";
import { filter, identity, mergeAll, mergeMap } from "rxjs";
import { MessengerViewEpicType } from "../types/plugin.js";
import { invoke_get } from "../../../library/invoke-get.js";
import { MessengerViewNameChangedAction, MessengerViewActionType } from "../actions/index.js";
import { AnyAction } from "redux";

export const initEpic: MessengerViewEpicType = (action$, state$, context) => action$.pipe(
    filter(isPluginInitializeAction),
    mergeMap(async () => {
        const actions: AnyAction[] = [];
        const roomCoreState = await invoke_get("1io", "room");
        const chatName = roomCoreState?.name;
        if (chatName) {
            actions.push(identity<MessengerViewNameChangedAction>({
                payload: { newName: chatName },
                type: MessengerViewActionType.NameChanged,
            }));
        }
        return actions;
    }),
    mergeAll(),
);
