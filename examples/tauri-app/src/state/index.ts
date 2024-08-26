import { PluginId } from "@1io/kui-application-sdk";

export interface ChatsListPluginState {
    readonly loadedPlugins: PluginId[];
    readonly activePlugin?: PluginId;
    readonly chats: string[];
}
