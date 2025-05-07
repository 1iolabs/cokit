import { GroupViewPluginActionType, GroupViewPluginActions } from "../actions/index.js";
import { GroupViewPluginState } from "../state/index.js";

export function groupViewPluginReducer(state: GroupViewPluginState, action: GroupViewPluginActions): GroupViewPluginState {
    switch (action.type) {
        case GroupViewPluginActionType.SetName:
            return { ...state, name: action.payload.name };
        case GroupViewPluginActionType.SetAvatar:
            return { ...state, avatar: action.payload.avatar };
        case GroupViewPluginActionType.ParticipantInvited:
            return { ...state, participants: [...state.participants, action.payload.participant] };
        case GroupViewPluginActionType.ParticipantRemoved:
            return { ...state, participants: state.participants.filter((p) => p !== action.payload.participant) }
        case GroupViewPluginActionType.Submit:
    }
    return state;
}
