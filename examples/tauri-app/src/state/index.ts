import { Message } from "@1io/coapp-messenger-view";

export interface MessengerPluginState {
    readonly messages: Message[];
    readonly chatName: string;
}
