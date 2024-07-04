import { Message } from "@1io/coapp-messenger-view";

export enum MessengerActionType {
    Send = "coapp-messenger/send",
    MessageReceived = "coapp-messenger/message-received",
}

export type MessengerActions = MessengerSendAction | MessageReceivedAction;

export interface MessengerSendAction {
    readonly payload: { message: string },
    readonly type: MessengerActionType.Send,
}

export interface MessageReceivedAction {
    readonly payload: { message: Message };
    readonly type: MessengerActionType.MessageReceived;
}
