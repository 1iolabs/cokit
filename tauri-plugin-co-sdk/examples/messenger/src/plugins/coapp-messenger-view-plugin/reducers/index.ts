import { MessengerViewActions, MessengerViewActionType } from "../actions/index.js";
import { MessengerViewPluginState } from "../state/index.js";

export function messengerViewReducer(state: MessengerViewPluginState, action: MessengerViewActions): MessengerViewPluginState {
    switch (action.type) {
        case MessengerViewActionType.MessageReceived: {
            // dedupe
            if (state.messages.findIndex((m) => m.key === action.payload.message.key) === -1) {
                if (action.payload.appendTop) {
                    return { ...state, messages: [action.payload.message, ...state.messages] };
                }
                return { ...state, messages: [...state.messages, action.payload.message] };
            }
            break;
        }
        case MessengerViewActionType.NameChanged: {
            return { ...state, chatName: action.payload.newName };
        }
        case MessengerViewActionType.SetLastHeads: {
            return { ...state, lastHeads: action.payload.lastHeads }
        }
    }
    return state;
}
