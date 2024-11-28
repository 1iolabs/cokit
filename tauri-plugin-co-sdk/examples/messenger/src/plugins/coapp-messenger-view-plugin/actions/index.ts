import { Message } from "@1io/coapp-messenger-view";
import { PayloadAction } from "@1io/redux-utils";
import { CID } from "multiformats";

export enum MessengerViewActionType {
    Send = "coapp-messenger/send",
    MessageReceived = "coapp-messenger/message-received",
    NameChanged = "coapp-messenger/chat-name-changed",
    LoadMoreEvents = "coapp-messenger/load-more-events",
    SetLastHeads = "coapp-messenger/set-last-heads",
}

export type MessengerViewActions =
    MessengerViewSendAction
    | MessengerViewAddMessageAction
    | MessengerViewNameChangedAction
    | MessengerViewLoadMoreEventsAction
    | MessengerViewSetLastHeadsAction;

export interface MessengerViewSendAction extends PayloadAction<MessengerViewActionType.Send, {
    readonly message: string;
}> {
}

export interface MessengerViewAddMessageAction extends PayloadAction<MessengerViewActionType.MessageReceived, {
    readonly message: Message;
    readonly appendTop?: boolean;
}> {
}

export interface MessengerViewNameChangedAction extends PayloadAction<MessengerViewActionType.NameChanged, {
    readonly newName: string;
}> {
}

export interface MessengerViewLoadMoreEventsAction extends PayloadAction<MessengerViewActionType.LoadMoreEvents, {
    readonly count: number;
}> {
}

export interface MessengerViewSetLastHeadsAction extends PayloadAction<MessengerViewActionType.SetLastHeads, {
    readonly lastHeads: Set<CID>;
}> {
}
