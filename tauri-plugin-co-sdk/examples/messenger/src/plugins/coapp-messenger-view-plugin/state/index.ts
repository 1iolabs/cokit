import { CID } from "multiformats";

export interface MessengerViewPluginState {
    readonly messages: CID[];
    readonly chatName: string;
    readonly co: string;
    readonly core: string;
    readonly lastHeads?: CID[];
}
