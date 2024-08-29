import { ChatsListActions, ChatsListActionType } from "../actions";
import { ChatsListPluginState } from "../state";

export function chatsListReducer(state: ChatsListPluginState | undefined, action: ChatsListActions): ChatsListPluginState {
    if (state === undefined) {
        return { chats: [], loadedPlugins: [] };
    }
    switch (action.type) {
        case ChatsListActionType.MessengerPluginLoaded: {
            return {
                ...state,
                loadedPlugins: [...state.loadedPlugins, action.payload.loadedPlugin]
            };
        }
        case ChatsListActionType.ActivatePlugin: {
            return { ...state, activePlugin: action.payload.pluginId };
        }
        case ChatsListActionType.SetChats: {
            return { ...state, chats: action.payload.chats };
        }
    }
    return state;
}
