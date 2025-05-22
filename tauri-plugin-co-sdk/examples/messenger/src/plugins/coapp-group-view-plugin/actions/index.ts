import { NotifyAction, PayloadAction } from "@1io/redux-utils";

export enum GroupViewPluginActionType {
    SetName = "coapp/group-view/setName",
    SetAvatar = "coapp/group-view/setAvatar",
    Submit = "coapp/group-view/submit",
    InviteParticipant = "coapp/group-view/inviteParticipant",
    RemoveParticipant = "coapp/group-view/removeParticipant",
    ParticipantInvited = "coapp/group-view/participantInvited",
    ParticipantRemoved = "coapp/group-view/participantRemoved",
    LoadProfilePicEpic = "coapp/group-view/loadProfilePicEpic",
}

export type GroupViewPluginActions =
    GroupViewSetNameAction
    | GroupViewSetAvatarAction
    | GroupViewSubmitAction
    | GroupViewInviteParticipantAction
    | GroupViewRemoveParticipantAction
    | GroupViewParticipantInvitedAction
    | GroupViewParticipantRemovedAction
    | GroupViewLoadProfilePicAction
    ;

export interface GroupViewSetNameAction extends PayloadAction<GroupViewPluginActionType.SetName, {
    readonly name: string;
}> { }
export interface GroupViewSetAvatarAction extends PayloadAction<GroupViewPluginActionType.SetAvatar, {
    readonly avatar?: string;
}> { }
export interface GroupViewSubmitAction extends NotifyAction<GroupViewPluginActionType.Submit> { }
export interface GroupViewInviteParticipantAction extends NotifyAction<GroupViewPluginActionType.InviteParticipant> { }
export interface GroupViewRemoveParticipantAction extends PayloadAction<GroupViewPluginActionType.RemoveParticipant, {
    readonly participant: string;
    readonly isYou: boolean;
}> { }
export interface GroupViewParticipantInvitedAction extends PayloadAction<GroupViewPluginActionType.ParticipantInvited, {
    readonly participant: string;
}> { }
export interface GroupViewParticipantRemovedAction extends PayloadAction<GroupViewPluginActionType.ParticipantRemoved, {
    readonly participant: string;
}> { }
export interface GroupViewLoadProfilePicAction extends NotifyAction<GroupViewPluginActionType.LoadProfilePicEpic> { }
