import { GlobalAction, PluginApi, PluginContext, PluginId, TagList, toGlobalAction } from "@1io/kui-application-sdk";
import { identity } from "rxjs";
import { ChatsListActionType, ChatsListOpenChatDetailsAction, ChatsListSetDialogAction } from "../actions/index.js";

export interface COAppChatsListApi extends PluginApi {
  loadDialog(plugin: string, pluginTags: TagList): Promise<[GlobalAction, PluginId]>;
  unloadDialog(): GlobalAction;
  openGroupView(coCoreId?: string): GlobalAction;
}

export function coappChatsListApi(context: PluginContext): COAppChatsListApi {
  return {
    loadDialog: async (plugin: string, pluginTags: TagList) => {
      const pluginInfo = await context.api.loadPlugin(plugin, pluginTags);
      return [
        toGlobalAction(
          context,
          identity<ChatsListSetDialogAction>({
            payload: { dialogPluginId: pluginInfo.id },
            type: ChatsListActionType.SetDialog,
          }),
        ),
        pluginInfo.id,
      ];
    },
    openGroupView: (coCoreId: string) => {
      return toGlobalAction(
        context,
        identity<ChatsListOpenChatDetailsAction>({
          payload: { coCoreId },
          type: ChatsListActionType.OpenChatDetails,
        }),
      );
    },
    unloadDialog: () => {
      return toGlobalAction(
        context,
        identity<ChatsListSetDialogAction>({
          payload: { dialogPluginId: undefined },
          type: ChatsListActionType.SetDialog,
        }),
      );
    },
  };
}
