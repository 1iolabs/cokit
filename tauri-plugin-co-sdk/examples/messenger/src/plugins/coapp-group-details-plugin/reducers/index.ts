import { GroupViewPluginActionType, GroupViewPluginActions } from "../actions/index.js";
import { GroupViewPluginState } from "../state/index.js";

export function groupViewPluginReducer(state: GroupViewPluginState, action: GroupViewPluginActions): GroupViewPluginState {
    switch (action.type) {
        case GroupViewPluginActionType.SetName:
            return { ...state, name: action.payload.name };
        case GroupViewPluginActionType.SetAvatar:
            return { ...state, avatar: action.payload.avatar };
        case GroupViewPluginActionType.SetParticipants:
        case GroupViewPluginActionType.Submit:
        case GroupViewPluginActionType.RemoveParticipant:
    }
    return state;
}
