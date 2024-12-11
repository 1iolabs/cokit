import { MessengerViewActions, MessengerViewActionType } from "../actions/index.js";
import { MessengerViewPluginState } from "../state/index.js";

export function messengerViewReducer(state: MessengerViewPluginState, action: MessengerViewActions): MessengerViewPluginState {
    switch (action.type) {
        case MessengerViewActionType.AddMessages: {
            // filter messages for duplicates already in state
            const filterdMessages = action.payload.messages.filter((m) => !state.messages.includes(m));
            if (action.payload.appendTop) {
                // first item is at the top, so we add new messages in beginning
                return { ...state, messages: filterdMessages.concat(state.messages) };
            }
            // add new items at the end
            return { ...state, messages: state.messages.concat(filterdMessages) };
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
