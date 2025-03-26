import { Chat } from "@1io/coapp-chatlist-view";
import { PluginId } from "@1io/kui-application-sdk";

export interface ChatsListPluginState {
    readonly activePlugin?: PluginId;
    readonly chats: Chat[];
    readonly loadedChats: Map<string, PluginId>;
}
