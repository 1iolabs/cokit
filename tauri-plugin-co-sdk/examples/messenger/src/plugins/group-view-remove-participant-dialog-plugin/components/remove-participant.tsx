import { RemoveParticipantDialog } from "@1io/coapp-chatlist-view";
import React from "react";
import { useDispatch } from "react-redux";
import { DialogProps } from "../../../types/dialog-container-props.js";
import { RemoveParticipantDialogActionType, RemoveParticipantDialogRemoveAction } from "../actions/index.js";

export interface RemoveParticipantDialogContainer extends DialogProps {
    readonly groupName: string;
    readonly did: string;
}

export function RemoveParticipantDialogContainer(props: RemoveParticipantDialogContainer) {
    const dispatch = useDispatch();
    const onClickRemoveParticipant = () => {
        dispatch<RemoveParticipantDialogRemoveAction>({
            payload: { did: props.did },
            public: true,
            type: RemoveParticipantDialogActionType.Remove,
        });
        props.onClose();
    }
    return <RemoveParticipantDialog
        onBack={props.onClose}
        groupName={props.groupName}
        onClickRemoveParticipant={onClickRemoveParticipant}
        participant={props.did}
    />;
}
