import { isPluginInitializeAction, BaseApi } from "@1io/kui-application-sdk";
import { filter, identity, mergeAll, mergeMap } from "rxjs";
import { MessengerEpicType } from "../types/plugin";
import { invoke_get } from "../library/invoke-get";
import { ChatNameChangedAction, MessengerActionType } from "../actions";
import { AnyAction } from "redux";

export const initEpic: MessengerEpicType = (action$, state$, context) => action$.pipe(
    filter(isPluginInitializeAction),
    mergeMap(async () => {
        const actions: AnyAction[] = [];
        const baseApi = context.api.getApi<BaseApi>([{ key: "type", value: "base" }]);
        actions.push(baseApi.set(
            context.plugin,
            [
                { key: "coapp-messenger", value: context.plugin },
            ],
        ));
        const roomCoreState = await invoke_get("1io", "room");
        const chatName = roomCoreState?.name;
        if (chatName) {
            actions.push(identity<ChatNameChangedAction>({
                payload: { newName: chatName },
                type: MessengerActionType.ChatNameChanged,
            }));
        }
        return actions;
    }),
    mergeAll(),
);
