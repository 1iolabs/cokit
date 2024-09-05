import { PluginId } from "@1io/kui-application-sdk";
import { Chat, LoadedCorePlugin } from "../state";

export enum ChatsListActionType {
    SetChats = "coapp/chatsList/setChats",
    OpenChat = "coapp/chats-list/openChat",
    MessengerPluginLoaded = "coapp/chats-list/messengerPluginLoaded",
    ActivatePlugin = "coapp/chats-list/activatePlugin",
    UpdateChat = "coapp/chats-list/updateChat",
    RemoveChat = "coapp/chats-list/removeChat",
}

export type ChatsListActions = ChatsListOpenChatAction | ChatsListMessengerPluginLoadedAction
    | ChatsListActivatePluginAction | ChatsListSetChatsAction | ChatsListUpdateChatAction;

export interface ChatsListSetChatsAction {
    readonly payload: { chats: Chat[] };
    readonly type: ChatsListActionType.SetChats;
}

export interface ChatsListOpenChatAction {
    readonly payload: { chat: Chat };
    readonly type: ChatsListActionType.OpenChat;
}

export interface ChatsListMessengerPluginLoadedAction {
    readonly payload: { loadedPlugin: LoadedCorePlugin };
    readonly type: ChatsListActionType.MessengerPluginLoaded;

}

export interface ChatsListActivatePluginAction {
    readonly payload: { pluginId?: PluginId };
    readonly type: ChatsListActionType.ActivatePlugin;
}

export interface ChatsListUpdateChatAction {
    readonly payload: {
        chat: Partial<Chat>;
        roomCoreId: string;
    };
    readonly type: ChatsListActionType.UpdateChat;
}