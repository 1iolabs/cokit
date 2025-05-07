import { Chat } from "@1io/coapp-chatlist-view";
import { PluginId } from "@1io/kui-application-sdk";
import { NotifyAction, PayloadAction } from "@1io/redux-utils";

export enum ChatsListActionType {
    SetChats = "coapp/chatsList/set-chats",
    OpenChat = "coapp/chats-list/open-chat",
    UpdateChat = "coapp/chats-list/update-chat",
    RemoveChat = "coapp/chats-list/remove-chat",
    ChatPluginLoaded = "coapp/chats-list/chat-plugin-loaded",
    OpenChatDetails = "coapp/chats-list/open-chat-details",
    SetPriorityPlugin = "coapp/chats-list/set-priority-plugin",
    SetIdentity = "coapp/chats-list/set-identity",
    CopyIdentity = "coapp/chats-list/copy-identity",
    SetDialog = "coapp/chats-list/set-dialog",
}

export type ChatsListActions = ChatsListOpenChatAction
    | ChatsListSetChatsAction
    | ChatsListUpdateChatAction
    | ChatsListChatPluginLoaded
    | ChatsListOpenChatDetailsAction
    | ChatsListSetPriorityPlugin
    | ChatsListSetIdentityAction
    | ChatsListCopyIdentityAction
    | ChatsListSetDialogAction;

export interface ChatsListSetChatsAction extends PayloadAction<ChatsListActionType.SetChats, {
    readonly chats: Chat[];
}
> { }

export interface ChatsListOpenChatAction extends PayloadAction<ChatsListActionType.OpenChat, {
    readonly chat: Chat;
}> { }

export interface ChatsListUpdateChatAction extends PayloadAction<ChatsListActionType.UpdateChat, {
    readonly chat: Partial<Chat> & Pick<Chat, "id">;
}> { }

export interface ChatsListChatPluginLoaded extends PayloadAction<ChatsListActionType.ChatPluginLoaded, {
    readonly chatId: string;
    readonly pluginId: PluginId;
}> { }

export interface ChatsListOpenChatDetailsAction extends PayloadAction<ChatsListActionType.OpenChatDetails, {
    readonly coCoreId?: string;
}> { }

export interface ChatsListSetPriorityPlugin extends PayloadAction<ChatsListActionType.SetPriorityPlugin, {
    readonly pluginId?: PluginId;
}> { }

export interface ChatsListSetIdentityAction extends PayloadAction<ChatsListActionType.SetIdentity, {
    readonly identity: string;
}> { }

export interface ChatsListCopyIdentityAction extends NotifyAction<ChatsListActionType.CopyIdentity> { }

export interface ChatsListSetDialogAction extends PayloadAction<ChatsListActionType.SetDialog, {
    readonly dialogPluginId?: PluginId;
}> { }
