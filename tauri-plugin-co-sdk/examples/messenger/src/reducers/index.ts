import { ChatsListActions, ChatsListActionType } from "../actions";
import { ChatsListPluginState } from "../state";

export function chatsListReducer(state: ChatsListPluginState | undefined, action: ChatsListActions): ChatsListPluginState {
    if (state === undefined) {
        return { chats: [] };
    }
    switch (action.type) {
        case ChatsListActionType.ActivatePlugin: {
            return { ...state, activePlugin: action.payload.pluginId };
        }
        case ChatsListActionType.SetChats: {
            return { ...state, chats: action.payload.chats };
        }
        case ChatsListActionType.UpdateChat: {
            return {
                ...state, chats: state.chats.map((chat) => {
                    if (chat.roomCoreId === action.payload.chat.roomCoreId) {
                        return { ...chat, ...action.payload.chat };
                    }
                    return chat;
                })
            }
        }
    }
    return state;
}
