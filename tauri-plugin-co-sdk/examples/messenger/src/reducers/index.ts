import { ChatsListActions, ChatsListActionType } from "../actions/index.js";
import { ChatsListPluginState } from "../state/index.js";

export function chatsListReducer(state: ChatsListPluginState | undefined, action: ChatsListActions): ChatsListPluginState {
    if (state === undefined) {
        return { chats: [], loadedChats: new Map };
    }
    switch (action.type) {
        case ChatsListActionType.SetChats:
            return { ...state, chats: action.payload.chats };
        case ChatsListActionType.UpdateChat:
            return {
                ...state,
                chats: state.chats.map((chat) => {
                    if (chat.id === action.payload.chat.id) {
                        return { ...chat, ...action.payload.chat };
                    }
                    return chat;
                }),
            }
        case ChatsListActionType.ChatPluginLoaded:
            return { ...state, loadedChats: state.loadedChats.set(action.payload.chatId, action.payload.pluginId) };
        case ChatsListActionType.OpenChat:
            return { ...state, selectedChat: action.payload.chat.id };
        case ChatsListActionType.SetPriorityPlugin:
            return { ...state, priorityPluginiId: action.payload.pluginId };
    }
    return state;
}
