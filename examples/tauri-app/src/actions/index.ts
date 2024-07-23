import { Message } from "@1io/coapp-messenger-view";

export enum MessengerActionType {
    Send = "coapp-messenger/send",
    MessageReceived = "coapp-messenger/message-received",
    ChatNameChanged = "coapp-messenger/chat-name-changed",
}

export type MessengerActions = MessengerSendAction | MessageReceivedAction | ChatNameChangedAction;

export interface MessengerSendAction {
    readonly payload: { message: string },
    readonly type: MessengerActionType.Send,
}

export interface MessageReceivedAction {
    readonly payload: { message: Message };
    readonly type: MessengerActionType.MessageReceived;
}

export interface ChatNameChangedAction {
    readonly payload: { newName: string },
    readonly type: MessengerActionType.ChatNameChanged,
}

