import { isPluginInitializeAction, WellKnownTags } from "@1io/kui-application-sdk";
import { filter, mergeMap } from "rxjs";
import { coappChatsListPluginId } from "../../coapp-chatslist-plugin/types/plugin.js";
import { MessengerViewEpicType } from "../types/plugin.js";

export const initEpic: MessengerViewEpicType = (action$, state$, context) =>
  action$.pipe(
    filter(isPluginInitializeAction),
    mergeMap(() => {
      // subscribe chats list public state
      return [
        context.api.subscribeState([{ key: WellKnownTags.Type, value: coappChatsListPluginId }], "chatsListState"),
      ];
    }),
  );
