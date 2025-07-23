import { InviteParticipantDialog } from "@1io/coapp-chatlist-view";
import React from "react";
import { useDispatch, useSelector } from "react-redux";
import { DialogProps } from "../../../types/dialog-container-props.js";
import {
  AddParticipantDialogActionType,
  AddParticipantDialogChangeDidAction,
  AddParticipantDialogSaveAction,
} from "../actions/index.js";
import { AddParticipantDialogPluginState } from "../types/state.js";

export function AddParticipantsDialogContainer(props: DialogProps) {
  const dispatch = useDispatch();
  const participantId = useSelector((state: AddParticipantDialogPluginState) => state.did);
  const onChangeParticipantId = (did: string) =>
    dispatch<AddParticipantDialogChangeDidAction>({
      payload: { did },
      type: AddParticipantDialogActionType.ChangeDid,
    });
  const onSaveParticipant = () => {
    // TODO validate
    dispatch<AddParticipantDialogSaveAction>({
      payload: { did: participantId },
      public: true,
      type: AddParticipantDialogActionType.Save,
    });
    props.onClose();
  };

  return (
    <InviteParticipantDialog
      participantId={participantId}
      onBack={props.onClose}
      onChangeParticipantId={onChangeParticipantId}
      onClickAddToGroup={onSaveParticipant}
    />
  );
}
