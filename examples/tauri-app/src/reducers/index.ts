import { MessengerActions, MessengerActionType } from "../actions";
import { MessengerPluginState } from "../state";

export function messengerReducer(state: MessengerPluginState | undefined, action: MessengerActions): MessengerPluginState {
    if (state === undefined) {
        return { messages: [], chatName: "" };
    }
    switch (action.type) {
        case MessengerActionType.MessageReceived: {
            return { ...state, messages: [...state.messages, action.payload.message] };
        }
        case MessengerActionType.ChatNameChanged: {
            return { ...state, chatName: action.payload.newName };
        }
    }
    return state;
}
