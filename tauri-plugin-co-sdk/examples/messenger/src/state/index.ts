import { PluginId } from "@1io/kui-application-sdk";

export interface Chat {
    readonly roomCoreId: string;
    readonly name: string;
    readonly lastMessage?: string;
    readonly newMessages: number;
    readonly pluginId?: PluginId;
}

export interface ChatsListPluginState {
    readonly activePlugin?: PluginId;
    readonly chats: Chat[];
}
