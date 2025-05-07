import { PluginPublicAction } from "@1io/kui-application-sdk";
import { PayloadAction } from "@1io/redux-utils";

export enum RemoveParticipantDialogActionType {
    Remove = "coapp/group-view/remove-participant-dialog/save",
}

export type RemoveParticipantDialogActions = RemoveParticipantDialogRemoveAction;

export type RemoveParticipantDialogRemoveAction = PayloadAction<RemoveParticipantDialogActionType.Remove, {
    readonly did: string;
}> & PluginPublicAction<RemoveParticipantDialogActionType.Remove>;
