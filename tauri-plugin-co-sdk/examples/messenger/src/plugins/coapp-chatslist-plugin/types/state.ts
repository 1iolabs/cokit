import { Chat } from "@1io/coapp-chatlist-view";
import { PluginId } from "@1io/kui-application-sdk";

export interface ChatsListPluginState {
    readonly chats: Chat[];
    readonly loadedChats: Map<string, PluginId>;
    readonly selectedChat?: string;
    readonly priorityPluginiId?: PluginId;
    readonly identity?: string;
    readonly dialog?: PluginId;
}

export interface ChatsListPluginPublicState extends Pick<ChatsListPluginState, "identity"> { }
