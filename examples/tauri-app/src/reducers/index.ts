import { MessengerActions, MessengerActionType } from "../actions";
import { MessengerPluginState } from "../state";

export function messengerReducer(state: MessengerPluginState | undefined, action: MessengerActions): MessengerPluginState {
    if (state === undefined) {
        return { messages: [] };
    }
    switch (action.type) {
        case MessengerActionType.MessageReceived: {
            return { ...state, messages: [...state.messages, action.payload.message] };
        }
    }
    return state;
}
