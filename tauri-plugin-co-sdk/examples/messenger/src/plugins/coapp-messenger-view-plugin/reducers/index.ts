import { MessengerViewActions, MessengerViewActionType } from "../actions";
import { MessengerViewPluginState } from "../state";

export function messengerViewReducer(state: MessengerViewPluginState, action: MessengerViewActions): MessengerViewPluginState {
    switch (action.type) {
        case MessengerViewActionType.MessageReceived: {
            if (state.messages.findIndex((m) => m.actionCid === action.payload.message.actionCid) == -1) {
                return { ...state, messages: [...state.messages, action.payload.message] };
            }
            break;
        }
        case MessengerViewActionType.NameChanged: {
            return { ...state, chatName: action.payload.newName };
        }
    }
    return state;
}
