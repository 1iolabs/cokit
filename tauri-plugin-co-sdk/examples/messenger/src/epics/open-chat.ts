import { ApplicationApi, WellKnownTags } from "@1io/kui-application-sdk";
import { filter, identity, mergeAll, mergeMap, withLatestFrom } from "rxjs";
import { ChatsListActionType, ChatsListActivatePluginAction, ChatsListOpenChatAction, ChatsListUpdateChatAction } from "../actions/index.js";
import { ChatsListEpicType } from "../types/plugin.js";

export const openChatEpic: ChatsListEpicType = (action$, state$, context) => action$.pipe(
    filter((action): action is ChatsListOpenChatAction => action.type === ChatsListActionType.OpenChat),
    withLatestFrom(state$),
    mergeMap(async ([action, state]) => {
        console.log("testtyy", action.payload.chat);
        const roomCoreId = action.payload.chat.roomCoreId;
        const loadedPlugin = action.payload.chat.pluginId;
        // if plugin not loaded -> load now
        if (loadedPlugin === undefined) {
            const applicationApi = context.api.getApi<ApplicationApi>([{ key: WellKnownTags.Type, value: "application" }]);
            const pluginInfo = await applicationApi.loadPlugin("coapp-messenger-view", [
                { key: "coreId", value: roomCoreId },
            ]);
            return [
                identity<ChatsListActivatePluginAction>({
                    payload: { pluginId: pluginInfo.id },
                    type: ChatsListActionType.ActivatePlugin,
                }),
                identity<ChatsListUpdateChatAction>({
                    payload: { chat: { roomCoreId, newMessages: 0, pluginId: pluginInfo.id } },
                    type: ChatsListActionType.UpdateChat,
                }),
            ];
        } else {
            return [
                identity<ChatsListActivatePluginAction>({
                    payload: { pluginId: loadedPlugin },
                    type: ChatsListActionType.ActivatePlugin,
                }),
                identity<ChatsListUpdateChatAction>({
                    payload: { chat: { roomCoreId, newMessages: 0 } },
                    type: ChatsListActionType.UpdateChat,
                }),
            ];
        }
    }),
    mergeAll(),
);