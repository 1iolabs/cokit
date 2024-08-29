import { ContextPlugin, EpicPlugin, Plugin, PluginContext, PluginEpicType, RenderPlugin } from "@1io/kui-application-sdk";
import { MessengerViewActions } from "../actions";
import { MessengerViewContainerProps } from "../components/chat-view";
import { MessengerViewPluginState } from "../state";

export type MessengerViewEpicType = PluginEpicType<MessengerViewPlugin>;

export type MessengerViewPlugin = Plugin<MessengerViewPluginState, MessengerViewActions> & RenderPlugin<MessengerViewContainerProps> & EpicPlugin<MessengerViewPluginState> & ContextPlugin<MessengerViewPluginContext>;

export interface MessengerViewPluginContext extends PluginContext { }
