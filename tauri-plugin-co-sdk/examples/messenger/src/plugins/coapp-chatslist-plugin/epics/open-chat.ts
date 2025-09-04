import { ApplicationApi, WellKnownTags } from "@1io/kui-application-sdk";
import { EMPTY, filter, identity, mergeAll, mergeMap, withLatestFrom } from "rxjs";
import { ChatsListActionType, ChatsListChatPluginLoaded, ChatsListOpenChatAction } from "../actions/index.js";
import { ChatsListEpicType } from "../types/plugin.js";
import { CoCoreIdTag } from "../../coapp-messenger-view-plugin/types/tags.js";

export const openChatEpic: ChatsListEpicType = (action$, state$, context) =>
  action$.pipe(
    filter((action): action is ChatsListOpenChatAction => action.type === ChatsListActionType.OpenChat),
    withLatestFrom(state$),
    mergeMap(async ([action, state]) => {
      const chatId = action.payload.chat;
      const loadedPlugin = undefined; //state.loadedChats.get(chatId);
      // if plugin not loaded -> load now
      if (loadedPlugin === undefined) {
        const applicationApi = context.api.getApi<ApplicationApi>([{ key: WellKnownTags.Type, value: "application" }]);
        const coCoreTag: CoCoreIdTag = { key: "coCoreId", value: chatId };
        const pluginInfo = await applicationApi.loadPlugin("coapp-messenger-view", [coCoreTag]);
        return [
          identity<ChatsListChatPluginLoaded>({
            payload: { chatId, pluginId: pluginInfo.id },
            type: ChatsListActionType.ChatPluginLoaded,
          }),
        ];
      }
      return EMPTY;
    }),
    mergeAll(),
  );
