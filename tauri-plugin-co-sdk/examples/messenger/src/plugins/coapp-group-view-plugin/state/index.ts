import { ChatsListPluginPublicState } from "../../coapp-chatslist-plugin/types/state.js";

export interface GroupViewPluginState {
    readonly name: string;
    readonly isNew: boolean;
    readonly avatar?: string;
    readonly participants: ReadonlyArray<string>;
    readonly chatsListState?: ChatsListPluginPublicState;
}
