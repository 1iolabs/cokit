import { ContextPlugin, EpicPlugin, Plugin, PluginContext, PluginEpicType, RenderPlugin } from "@1io/kui-application-sdk";
import { ChatsListActions } from "../actions/index.js";
import { ChatsListPluginState } from "../state/index.js";

export type ChatsListEpicType = PluginEpicType<ChatsListPlugin>;

export type ChatsListPlugin = Plugin<ChatsListPluginState, ChatsListActions> & RenderPlugin & EpicPlugin<ChatsListPluginState> & ContextPlugin<ChatsListPluginContext>;

export interface ChatsListPluginContext extends PluginContext { }
