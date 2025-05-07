import { ApiPlugin, ContextPlugin, EpicPlugin, Plugin, PluginContext, PluginEpicType, PublicStatePlugin, RenderPlugin } from "@1io/kui-application-sdk";
import { ChatsListActions } from "../actions/index.js";
import { COAppChatsListApi } from "../api/index.js";
import { ChatsListPluginPublicState, ChatsListPluginState } from "./state.js";

export const coappChatsListPluginId = "coapp-chats-list";

export type ChatsListEpicType = PluginEpicType<ChatsListPlugin>;

export type ChatsListPlugin = Plugin<ChatsListPluginState, ChatsListActions>
    & RenderPlugin
    & EpicPlugin<ChatsListPluginState>
    & ContextPlugin<ChatsListPluginContext>
    & ApiPlugin<COAppChatsListApi>
    & PublicStatePlugin<ChatsListPluginState, ChatsListPluginPublicState>;

export interface ChatsListPluginContext extends PluginContext { }
