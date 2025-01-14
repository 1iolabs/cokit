import { ContextPlugin, EpicPlugin, Plugin, PluginContext, PluginEpicType, RenderPlugin } from "@1io/kui-application-sdk";
import { MessengerViewActions } from "../actions/index.js";
import { MessengerViewContainerProps } from "../components/chat-view.js";
import { MessengerViewPluginState } from "../state/index.js";

export type MessengerViewEpicType = PluginEpicType<MessengerViewPlugin>;

export type MessengerViewPlugin = Plugin<MessengerViewPluginState, MessengerViewActions> & RenderPlugin<MessengerViewContainerProps> & EpicPlugin<MessengerViewPluginState> & ContextPlugin<MessengerViewPluginContext>;

export interface MessengerViewPluginContext extends PluginContext { }
