import { PayloadAction } from "@1io/redux-utils";

export enum MessengerViewActionType {
  Send = "coapp-messenger/send",
}

export type MessengerViewActions = MessengerViewSendAction;

export interface MessengerViewSendAction
  extends PayloadAction<
    MessengerViewActionType.Send,
    {
      readonly message: string;
    }
  > {}
