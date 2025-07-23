import { ChatsListPluginPublicState } from "../../coapp-chatslist-plugin/types/state.js";

export interface MessengerViewPluginState {
  readonly coId: string;
  readonly coreId: string;
  readonly chatsListState?: ChatsListPluginPublicState;
}
