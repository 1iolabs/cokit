import { Chat } from "@1io/coapp-chatlist-view";
import { PluginId } from "@1io/kui-application-sdk";
import { PayloadAction } from "@1io/redux-utils";

export enum ChatsListActionType {
    SetChats = "coapp/chatsList/setChats",
    OpenChat = "coapp/chats-list/openChat",
    ActivatePlugin = "coapp/chats-list/activatePlugin",
    UpdateChat = "coapp/chats-list/updateChat",
    RemoveChat = "coapp/chats-list/removeChat",
    ChatPluginLoaded = "coapp/chats-list/chatPluginLoaded",
}

export type ChatsListActions = ChatsListOpenChatAction | ChatsListActivatePluginAction
    | ChatsListSetChatsAction | ChatsListUpdateChatAction | ChatsListChatPluginLoaded;

export interface ChatsListSetChatsAction {
    readonly payload: { chats: Chat[] };
    readonly type: ChatsListActionType.SetChats;
}

export interface ChatsListOpenChatAction {
    readonly payload: { chat: Chat };
    readonly type: ChatsListActionType.OpenChat;
}

export interface ChatsListActivatePluginAction {
    readonly payload: { pluginId?: PluginId };
    readonly type: ChatsListActionType.ActivatePlugin;
}

export interface ChatsListUpdateChatAction {
    readonly payload: {
        chat: Partial<Chat> & Pick<Chat, "id">;
    };
    readonly type: ChatsListActionType.UpdateChat;
}

export interface ChatsListChatPluginLoaded extends PayloadAction<ChatsListActionType.ChatPluginLoaded, {
    readonly chatId: string;
    readonly pluginId: PluginId;
}> {
}
