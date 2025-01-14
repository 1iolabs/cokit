import { isPluginInitializeAction } from "@1io/kui-application-sdk";
import { AnyAction } from "redux";
import { filter, identity, mergeAll, mergeMap, withLatestFrom } from "rxjs";
import { invokeGetCoreState } from "../../../library/invoke-get.js";
import { MessengerViewActionType, MessengerViewNameChangedAction } from "../actions/index.js";
import { MessengerViewEpicType } from "../types/plugin.js";

export const initEpic: MessengerViewEpicType = (action$, state$, context) => action$.pipe(
    filter(isPluginInitializeAction),
    withLatestFrom(state$),
    mergeMap(async ([, state]) => {
        const actions: AnyAction[] = [];
        // load core state of room
        const roomCoreState = await invokeGetCoreState(state.co, state.core);
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
