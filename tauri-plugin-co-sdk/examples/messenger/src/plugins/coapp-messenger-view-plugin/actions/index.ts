import { PayloadAction } from "@1io/redux-utils";
import { CID } from "multiformats";

export enum MessengerViewActionType {
    Send = "coapp-messenger/send",
    AddMessages = "coapp/messenger-view/add-messages",
    NameChanged = "coapp/messenger-view/chat-name-changed",
    LoadMoreEvents = "coapp/messenger-view/load-more-events",
    SetLastHeads = "coapp/messenger-view/set-last-heads",
    SetSession = "coapp/messenger-view/set-session",
}

export type MessengerViewActions =
    MessengerViewSendAction
    | MessengerViewAddMessagesAction
    | MessengerViewNameChangedAction
    | MessengerViewLoadMoreEventsAction
    | MessengerViewSetLastHeadsAction
    | MessengerViewSetSessionAction;

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

export interface MessengerViewSetSessionAction extends PayloadAction<MessengerViewActionType.SetSession, {
    readonly sessionId: string;
}> {
}
