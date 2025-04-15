import { Chat } from "@1io/coapp-chatlist-view";
import { PluginId } from "@1io/kui-application-sdk";
import { PayloadAction } from "@1io/redux-utils";

export enum ChatsListActionType {
    SetChats = "coapp/chatsList/setChats",
    OpenChat = "coapp/chats-list/openChat",
    UpdateChat = "coapp/chats-list/updateChat",
    RemoveChat = "coapp/chats-list/removeChat",
    ChatPluginLoaded = "coapp/chats-list/chatPluginLoaded",
    OpenChatDetails = "coapp/chats-list/openChatDetails",
    SetPriorityPlugin = "coapp/chats-list/setPriorityPlugin",
    SetIdentity = "coapp/chats-list/setIdentity",
}

export type ChatsListActions = ChatsListOpenChatAction
    | ChatsListSetChatsAction
    | ChatsListUpdateChatAction
    | ChatsListChatPluginLoaded
    | ChatsListOpenChatDetailsAction
    | ChatsListSetPriorityPlugin
    | ChatsListSetIdentityAction;

export interface ChatsListSetChatsAction {
    readonly payload: { chats: Chat[] };
    readonly type: ChatsListActionType.SetChats;
}

export interface ChatsListOpenChatAction {
    readonly payload: { chat: Chat };
    readonly type: ChatsListActionType.OpenChat;
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

export interface ChatsListOpenChatDetailsAction extends PayloadAction<ChatsListActionType.OpenChatDetails, {
    readonly coCoreId?: string;
}> { }

export interface ChatsListSetPriorityPlugin extends PayloadAction<ChatsListActionType.SetPriorityPlugin, {
    readonly pluginId?: PluginId;
}> { }

export interface ChatsListSetIdentityAction extends PayloadAction<ChatsListActionType.SetIdentity, {
    readonly identity: string;
}> { }
