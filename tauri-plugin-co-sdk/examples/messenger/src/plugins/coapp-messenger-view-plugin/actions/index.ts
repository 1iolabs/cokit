import { PayloadAction } from "@1io/redux-utils";
import { CID } from "multiformats";

export enum MessengerViewActionType {
    Send = "coapp-messenger/send",
    AddMessages = "coapp-messenger/add-messages",
    NameChanged = "coapp-messenger/chat-name-changed",
    LoadMoreEvents = "coapp-messenger/load-more-events",
    SetLastHeads = "coapp-messenger/set-last-heads",
}

export type MessengerViewActions =
    MessengerViewSendAction
    | MessengerViewAddMessagesAction
    | MessengerViewNameChangedAction
    | MessengerViewLoadMoreEventsAction
    | MessengerViewSetLastHeadsAction;

export interface MessengerViewSendAction extends PayloadAction<MessengerViewActionType.Send, {
    readonly message: string;
}> {
}

export interface MessengerViewAddMessagesAction extends PayloadAction<MessengerViewActionType.AddMessages, {
    readonly messages: CID[];
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
    readonly lastHeads: CID[];
}> {
}
