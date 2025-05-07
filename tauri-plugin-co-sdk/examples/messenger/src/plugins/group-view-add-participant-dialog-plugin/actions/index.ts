import { PluginPublicAction } from "@1io/kui-application-sdk";
import { PayloadAction } from "@1io/redux-utils";

export enum AddParticipantDialogActionType {
    ChangeDid = "coapp/add-participant-dialog/change-did",
    Save = "coapp/add-participant-dialog/save",
}

export type AddParticipantDialogActions = AddParticipantDialogChangeDidAction | AddParticipantDialogSaveAction;

export interface AddParticipantDialogChangeDidAction extends PayloadAction<AddParticipantDialogActionType.ChangeDid, {
    readonly did: string;
}> { }

export interface AddParticipantDialogSaveAction extends PluginPublicAction<AddParticipantDialogActionType.Save>, PayloadAction<AddParticipantDialogActionType.Save, {
    readonly did: string;
}> { }
