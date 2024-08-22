export enum ChatsListActionType {
    OpenChat = "coapp/chats-list/openChat",
}

export type ChatsListActions = ChatsListOpenChatAction;

export interface ChatsListOpenChatAction {
    readonly payload: { chat: string },
    readonly type: ChatsListActionType.OpenChat,
}
