import { PluginId } from "@1io/kui-application-sdk";
import { NotifyAction, PayloadAction } from "@1io/redux-utils";
import { LoadedChatPlugin } from "../types/state";

export enum ChatsListActionType {
  OpenChat = "coapp/chats-list/open-chat",
  ChatPluginLoaded = "coapp/chats-list/chat-plugin-loaded",
  OpenChatDetails = "coapp/chats-list/open-chat-details",
  SetPriorityPlugin = "coapp/chats-list/set-priority-plugin",
  SetIdentity = "coapp/chats-list/set-identity",
  CopyIdentity = "coapp/chats-list/copy-identity",
  SetDialog = "coapp/chats-list/set-dialog",
}

export type ChatsListActions =
  | ChatsListOpenChatAction
  | ChatsListChatPluginLoaded
  | ChatsListOpenChatDetailsAction
  | ChatsListSetPriorityPlugin
  | ChatsListSetIdentityAction
  | ChatsListCopyIdentityAction
  | ChatsListSetDialogAction;

export interface ChatsListOpenChatAction
  extends PayloadAction<
    ChatsListActionType.OpenChat,
    {
      readonly chat: string;
    }
  > {}

export interface ChatsListChatPluginLoaded
  extends PayloadAction<
    ChatsListActionType.ChatPluginLoaded,
    {
      readonly loadedChat: LoadedChatPlugin;
    }
  > {}

export interface ChatsListOpenChatDetailsAction
  extends PayloadAction<
    ChatsListActionType.OpenChatDetails,
    {
      readonly coCoreId?: string;
    }
  > {}

export interface ChatsListSetPriorityPlugin
  extends PayloadAction<
    ChatsListActionType.SetPriorityPlugin,
    {
      readonly pluginId?: PluginId;
    }
  > {}

export interface ChatsListSetIdentityAction
  extends PayloadAction<
    ChatsListActionType.SetIdentity,
    {
      readonly identity: string;
    }
  > {}

export interface ChatsListCopyIdentityAction extends NotifyAction<ChatsListActionType.CopyIdentity> {}

export interface ChatsListSetDialogAction
  extends PayloadAction<
    ChatsListActionType.SetDialog,
    {
      readonly dialogPluginId?: PluginId;
    }
  > {}
