import { AddParticipantDialogActions, AddParticipantDialogActionType } from "../actions/index.js";
import { AddParticipantDialogPluginState } from "../types/state.js";

export function addParticipantsDialogReducer(
    state: AddParticipantDialogPluginState,
    action: AddParticipantDialogActions,
): AddParticipantDialogPluginState {
    switch (action.type) {
        case AddParticipantDialogActionType.ChangeDid: {
            return { did: action.payload.did };
        }
    }
    return state;
}
