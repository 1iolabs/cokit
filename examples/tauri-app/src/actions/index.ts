import { PluginId } from "@1io/kui-application-sdk";

export enum ChatsListActionType {
    OpenChat = "coapp/chats-list/openChat",
    MessengerPluginLoaded = "coapp/chats-list/messengerPluginLoaded",
    ActivatePlugin = "coapp/chats-list/activatePlugin",
}

export type ChatsListActions = ChatsListOpenChatAction | ChatsListMessengerPluginLoadedAction
    | ChatsListActivatePluginAction;

export interface ChatsListOpenChatAction {
    readonly payload: { chat: string },
    readonly type: ChatsListActionType.OpenChat,
}

export interface ChatsListMessengerPluginLoadedAction {
    readonly payload: { pluginId: PluginId },
    readonly type: ChatsListActionType.MessengerPluginLoaded

}

export interface ChatsListActivatePluginAction {
    readonly payload: { pluginId?: PluginId },
    readonly type: ChatsListActionType.ActivatePlugin,
}
