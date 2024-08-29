import { ApplicationApi, WellKnownTags } from "@1io/kui-application-sdk";
import { filter, identity, mergeAll, mergeMap, withLatestFrom } from "rxjs";
import { ChatsListActionType, ChatsListActivatePluginAction, ChatsListMessengerPluginLoadedAction, ChatsListOpenChatAction } from "../actions";
import { ChatsListEpicType } from "../types/plugin";

export const openChatEpic: ChatsListEpicType = (action$, state$, context) => action$.pipe(
    filter((action): action is ChatsListOpenChatAction => action.type === ChatsListActionType.OpenChat),
    withLatestFrom(state$),
    mergeMap(async ([action, state]) => {
        const chatCoreId = action.payload.chat.roomCoreId;
        const loadedPlugin = state.loadedPlugins.find((plugin) => {
            return plugin.chat.roomCoreId === chatCoreId;
        });
        // if plugin not loaded -> load now
        if (loadedPlugin === undefined) {
            const applicationApi = context.api.getApi<ApplicationApi>([{ key: WellKnownTags.Type, value: "application" }]);
            const pluginInfo = await applicationApi.loadPlugin("coapp-messenger-view", [
                { key: "coreId", value: chatCoreId },
            ]);
            return [
                identity<ChatsListMessengerPluginLoadedAction>({
                    payload: { loadedPlugin: { pluginId: pluginInfo.id, chat: action.payload.chat } },
                    type: ChatsListActionType.MessengerPluginLoaded
                }),
                identity<ChatsListActivatePluginAction>({
                    payload: { pluginId: pluginInfo.id },
                    type: ChatsListActionType.ActivatePlugin,
                }),
            ];
        } else {
            return [identity<ChatsListActivatePluginAction>({
                payload: { pluginId: loadedPlugin.pluginId },
                type: ChatsListActionType.ActivatePlugin,
            })];
        }
    }),
    mergeAll(),
);