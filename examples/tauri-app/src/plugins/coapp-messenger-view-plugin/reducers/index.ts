import { MessengerViewActions, MessengerViewActionType } from "../actions";
import { MessengerViewPluginState } from "../state";

export function messengerViewReducer(state: MessengerViewPluginState, action: MessengerViewActions): MessengerViewPluginState {
    switch (action.type) {
        case MessengerViewActionType.MessageReceived: {
            return { ...state, messages: [...state.messages, action.payload.message] };
        }
        case MessengerViewActionType.NameChanged: {
            return { ...state, chatName: action.payload.newName };
        }
    }
    return state;
}
