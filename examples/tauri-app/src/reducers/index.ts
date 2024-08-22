import { ChatsListActions } from "../actions";
import { ChatsListPluginState } from "../state";

export function chatsListReducer(state: ChatsListPluginState | undefined, action: ChatsListActions): ChatsListPluginState {
    if (state === undefined) {
        return { chats: [] };
    }
    switch (action.type) {
    }
    return state;
}
