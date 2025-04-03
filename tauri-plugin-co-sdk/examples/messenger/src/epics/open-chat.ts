import { ApplicationApi, WellKnownTags } from "@1io/kui-application-sdk";
import { filter, identity, mergeAll, mergeMap, withLatestFrom } from "rxjs";
import { ChatsListActionType, ChatsListChatPluginLoaded, ChatsListOpenChatAction, ChatsListUpdateChatAction } from "../actions/index.js";
import { ChatsListEpicType } from "../types/plugin.js";

export const openChatEpic: ChatsListEpicType = (action$, state$, context) => action$.pipe(
    filter((action): action is ChatsListOpenChatAction => action.type === ChatsListActionType.OpenChat),
    withLatestFrom(state$),
    mergeMap(async ([action, state]) => {
        const chatId = action.payload.chat.id;
        const loadedPlugin = state.loadedChats.get(chatId);
        // if plugin not loaded -> load now
        if (loadedPlugin === undefined) {
            const applicationApi = context.api.getApi<ApplicationApi>([{ key: WellKnownTags.Type, value: "application" }]);
            const pluginInfo = await applicationApi.loadPlugin("coapp-messenger-view", [
                { key: "coreId", value: chatId },
            ]);
            return [
                identity<ChatsListChatPluginLoaded>({
                    payload: { chatId: chatId, pluginId: pluginInfo.id },
                    type: ChatsListActionType.ChatPluginLoaded,
                }),
                identity<ChatsListUpdateChatAction>({
                    payload: { chat: { id: chatId, newMessages: 0 } },
                    type: ChatsListActionType.UpdateChat,
                }),
            ];
        } else {
            return [
                identity<ChatsListUpdateChatAction>({
                    payload: { chat: { id: chatId, newMessages: 0 } },
                    type: ChatsListActionType.UpdateChat,
                }),
            ];
        }
    }),
    mergeAll(),
);
