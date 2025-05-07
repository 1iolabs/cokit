import { CID } from "multiformats";
import { ChatsListPluginPublicState } from "../../coapp-chatslist-plugin/types/state.js";

export interface MessengerViewPluginState {
    readonly messages: CID[];
    readonly chatName: string;
    readonly co: string;
    readonly core: string;
    readonly lastHeads?: CID[];
    readonly coSessionId?: string;
    readonly chatsListState?: ChatsListPluginPublicState;
}
