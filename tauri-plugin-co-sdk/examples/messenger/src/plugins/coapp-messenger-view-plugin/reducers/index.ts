import { MessengerViewActions, MessengerViewActionType } from "../actions/index.js";
import { MessengerViewPluginState } from "../types/state.js";

export function messengerViewReducer(state: MessengerViewPluginState, action: MessengerViewActions): MessengerViewPluginState {
    switch (action.type) {
        case MessengerViewActionType.AddMessages: {
            // filter messages for duplicates already in state
            const filteredMessages = action.payload.messages.filter((m) => {
                return state.messages.find((ms) => {
                    return m.toString() === ms.toString();
                }) === undefined;
            });
            if (action.payload.appendTop) {
                // first item is at the top, so we add new messages in beginning
                return { ...state, messages: filteredMessages.concat(state.messages) };
            }
            // add new items at the end
            return { ...state, messages: state.messages.concat(filteredMessages) };
        }
        case MessengerViewActionType.NameChanged: {
            return { ...state, chatName: action.payload.newName };
        }
        case MessengerViewActionType.SetLastHeads: {
            return { ...state, lastHeads: action.payload.lastHeads };
        }
        case MessengerViewActionType.SetSession: {
            return { ...state, coSessionId: action.payload.sessionId };
        };
    }
    return state;
}
