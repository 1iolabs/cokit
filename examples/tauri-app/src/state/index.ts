import { PluginId } from "@1io/kui-application-sdk";

export interface Chat {
    readonly roomCoreId: string;
    readonly name: string;
    readonly lastMessage?: string;
    readonly newMessages: number;
}

export interface LoadedCorePlugin {
    readonly pluginId: PluginId;
    readonly chat: Chat;
}

export interface ChatsListPluginState {
    readonly loadedPlugins: LoadedCorePlugin[];
    readonly activePlugin?: PluginId;
    readonly chats: Chat[];
}
