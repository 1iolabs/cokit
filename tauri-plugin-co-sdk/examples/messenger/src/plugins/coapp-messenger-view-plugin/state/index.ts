import { Message } from "@1io/coapp-messenger-view";

export interface MessengerViewPluginState {
    readonly messages: Message[];
    readonly chatName: string;
    readonly co: string;
    readonly core: string;
}
