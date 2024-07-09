import { ContextPlugin, EpicPlugin, Plugin, PluginContext, PluginEpicType, RenderPlugin } from "@1io/kui-application-sdk";
import { MessengerActions } from "../actions";
import { MessengerPluginState } from "../state";

export type MessengerEpicType = PluginEpicType<MessengerPlugin>;

export type MessengerPlugin = Plugin<MessengerPluginState, MessengerActions> & RenderPlugin & EpicPlugin<MessengerPluginState> & ContextPlugin<MessengerPluginContext>;

export interface MessengerPluginContext extends PluginContext { }
