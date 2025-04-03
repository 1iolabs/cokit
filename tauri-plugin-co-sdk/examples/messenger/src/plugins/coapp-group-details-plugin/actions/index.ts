import { Participant } from "@1io/coapp-chatlist-view";
import { PayloadAction } from "@1io/redux-utils";

export enum GroupViewPluginActionType {
    SetName = "setName",
    SetAvatar = "setAvatar",
    SetParticipants = "setParticipants",
    Submit = "submit",
    RemoveParticipant = "removeParticipant",
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
    readonly avatar: string;
}> { }
export interface GroupViewSetParticipantsAction extends PayloadAction<GroupViewPluginActionType.SetParticipants, {
    readonly participants: ReadonlyArray<Participant>;
}> { }
export interface GroupViewRemoveParticipantAction extends PayloadAction<GroupViewPluginActionType.RemoveParticipant, {
    readonly participant: Participant;
}> { }
export interface GroupViewSubmitAction extends PayloadAction<GroupViewPluginActionType.Submit, {
}> { }
