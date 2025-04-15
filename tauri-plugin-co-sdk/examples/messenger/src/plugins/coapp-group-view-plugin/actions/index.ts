import { Participant } from "@1io/coapp-chatlist-view";
import { NotifyAction, PayloadAction } from "@1io/redux-utils";

export enum GroupViewPluginActionType {
    SetName = "coapp/group-view/setName",
    SetAvatar = "coapp/group-view/setAvatar",
    SetParticipants = "coapp/group-view/setParticipants",
    Submit = "coapp/group-view/submit",
    RemoveParticipant = "coapp/group-view/removeParticipant",
    LoadProfilePicEpic = "coapp/group-view/loadProfilePicEpic",
}

export type GroupViewPluginActions =
    GroupViewSetNameAction
    | GroupViewSetAvatarAction
    | GroupViewSetParticipantsAction
    | GroupViewRemoveParticipantAction
    | GroupViewSubmitAction;

export interface GroupViewSetNameAction extends PayloadAction<GroupViewPluginActionType.SetName, {
    readonly name: string;
}> { }
export interface GroupViewSetAvatarAction extends PayloadAction<GroupViewPluginActionType.SetAvatar, {
    readonly avatar?: string;
}> { }
export interface GroupViewSetParticipantsAction extends PayloadAction<GroupViewPluginActionType.SetParticipants, {
    readonly participants: ReadonlyArray<Participant>;
}> { }
export interface GroupViewRemoveParticipantAction extends PayloadAction<GroupViewPluginActionType.RemoveParticipant, {
    readonly participant: Participant;
}> { }
export interface GroupViewSubmitAction extends PayloadAction<GroupViewPluginActionType.Submit, {
}> { }
export interface GroupViewLoadProfilePicAction extends NotifyAction<GroupViewPluginActionType.LoadProfilePicEpic> { }
