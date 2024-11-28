import { Message } from "@1io/coapp-messenger-view";
import { CID } from "multiformats";

export interface MessengerViewPluginState {
    readonly messages: Message[];
    readonly chatName: string;
    readonly co: string;
    readonly core: string;
    readonly lastHeads?: Set<CID>;
}
