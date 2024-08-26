import { ApplicationApi, WellKnownTags } from "@1io/kui-application-sdk";
import { filter, identity, mergeAll, mergeMap } from "rxjs";
import { ChatsListActionType, ChatsListActivatePluginAction, ChatsListMessengerPluginLoadedAction } from "../actions";
import { ChatsListEpicType } from "../types/plugin";

export const openChatEpic: ChatsListEpicType = (action$, _, context) => action$.pipe(
    filter((action) => action.type === ChatsListActionType.OpenChat),
    mergeMap(async (action) => {
        const applicationApi = context.api.getApi<ApplicationApi>([{ key: WellKnownTags.Type, value: "application" }]);
        const pluginInfo = await applicationApi.loadPlugin("coapp-messenger-view", []);
        // TODO load messenger view plugin with clicked on chat room
        return [
            identity<ChatsListMessengerPluginLoadedAction>({
                payload: { pluginId: pluginInfo.id },
                type: ChatsListActionType.MessengerPluginLoaded
            }),
            identity<ChatsListActivatePluginAction>({
                payload: { pluginId: pluginInfo.id },
                type: ChatsListActionType.ActivatePlugin,
            }),
        ];
    }),
    mergeAll(),
);