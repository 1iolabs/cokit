import { EpicPlugin, Plugin, PluginEpicType, RenderPlugin } from "@1io/kui-application-sdk";
import { GroupViewPluginActions } from "../actions/index.js";
import { GroupViewContainerProps } from "../components/index.js";
import { GroupViewPluginState } from "../state/index.js";

export type GroupViewEpicType = PluginEpicType<GroupViewPlugin>;

export type GroupViewPlugin = Plugin<GroupViewPluginState, GroupViewPluginActions> &
  RenderPlugin<GroupViewContainerProps> &
  EpicPlugin<GroupViewPluginState>;
