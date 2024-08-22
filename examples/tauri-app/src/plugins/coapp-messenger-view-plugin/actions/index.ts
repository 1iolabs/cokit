import { Message } from "@1io/coapp-messenger-view";

export enum MessengerViewActionType {
    Send = "coapp-messenger/send",
    MessageReceived = "coapp-messenger/message-received",
    NameChanged = "coapp-messenger/chat-name-changed",
}

export type MessengerViewActions = MessengerViewSendAction | MessengerViewReceivedAction | MessengerViewNameChangedAction;

export interface MessengerViewSendAction {
    readonly payload: { message: string },
    readonly type: MessengerViewActionType.Send,
}

export interface MessengerViewReceivedAction {
    readonly payload: { message: Message };
    readonly type: MessengerViewActionType.MessageReceived;
}

export interface MessengerViewNameChangedAction {
    readonly payload: { newName: string },
    readonly type: MessengerViewActionType.NameChanged,
}

