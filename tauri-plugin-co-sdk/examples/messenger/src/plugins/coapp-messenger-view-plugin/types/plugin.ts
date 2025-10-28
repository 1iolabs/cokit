import {
  ContextPlugin,
  EpicPlugin,
  Plugin,
  PluginContext,
  PluginEpicType,
  RenderPlugin,
} from "@1io/kui-application-sdk";
import { MessengerViewContainerProps } from "../components/chat-view.js";
import { MessengerViewPluginState } from "./state.js";
import { AnyAction } from "redux";

export type MessengerViewEpicType = PluginEpicType<MessengerViewPlugin>;

export type MessengerViewPlugin = Plugin<MessengerViewPluginState, AnyAction> &
  RenderPlugin<MessengerViewContainerProps> &
  EpicPlugin<MessengerViewPluginState, MessengerViewPluginContext, AnyAction> &
  ContextPlugin<MessengerViewPluginContext>;

export interface MessengerViewPluginContext extends PluginContext {}
