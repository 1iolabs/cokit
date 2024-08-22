import { ContextPlugin, EpicPlugin, Plugin, PluginContext, PluginEpicType, RenderPlugin } from "@1io/kui-application-sdk";
import { ChatsListActions } from "../actions";
import { ChatsListPluginState } from "../state";

export type ChatsListEpicType = PluginEpicType<ChatsListPlugin>;

export type ChatsListPlugin = Plugin<ChatsListPluginState, ChatsListActions> & RenderPlugin & EpicPlugin<ChatsListPluginState> & ContextPlugin<ChatsListPluginContext>;

export interface ChatsListPluginContext extends PluginContext { }
