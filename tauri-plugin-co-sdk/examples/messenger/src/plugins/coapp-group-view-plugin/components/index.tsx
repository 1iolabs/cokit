import { EditGroupView, NewGroupView, Participant } from "@1io/coapp-chatlist-view";
import React from "react";
import { useDispatch, useSelector } from "react-redux";
import ParticipantIcon from "../../../assets/User.svg";
import DefaultAvatar from "../../../assets/Users_48.svg";
import {
  GroupViewInviteParticipantAction,
  GroupViewLoadProfilePicAction,
  GroupViewPluginActionType,
  GroupViewRemoveParticipantAction,
  GroupViewSetNameAction,
  GroupViewSubmitAction,
} from "../actions/index.js";
import { GroupViewPluginState } from "../state/index.js";
export interface GroupViewContainerProps {
  readonly onClose: () => void;
}

export function GroupViewContainer(props: GroupViewContainerProps) {
  const dispatch = useDispatch();
  const groupName = useSelector((state: GroupViewPluginState) => state.name);
  const avatar = useSelector((state: GroupViewPluginState) => state.avatar);
  const isNew = useSelector((state: GroupViewPluginState) => state.isNew);
  const participantDids = useSelector((state: GroupViewPluginState) => state.participants);
  const ownIdentity = useSelector((state: GroupViewPluginState) => state.chatsListState?.identity);

  // TODO use converter
  const participants = participantDids.map(
    (did): Participant => ({
      avatar: ParticipantIcon, // TODO load information by did
      did,
      isYou: did === ownIdentity,
      menuItems: [
        did === ownIdentity
          ? {
              key: "leave",
              label: "Leave group",
              onTrigger: () =>
                dispatch<GroupViewRemoveParticipantAction>({
                  payload: { participant: did, isYou: true },
                  type: GroupViewPluginActionType.RemoveParticipant,
                }),
            }
          : {
              key: "remove",
              label: "Remove participant",
              onTrigger: () =>
                dispatch<GroupViewRemoveParticipantAction>({
                  payload: { participant: did, isYou: false },
                  type: GroupViewPluginActionType.RemoveParticipant,
                }),
            },
      ],
    }),
  );

  const onChangeGroupName = (name: string) => {
    dispatch<GroupViewSetNameAction>({ payload: { name }, type: GroupViewPluginActionType.SetName });
  };
  const onChangeAvatar = () =>
    dispatch<GroupViewLoadProfilePicAction>({ type: GroupViewPluginActionType.LoadProfilePicEpic });
  const onInviteParticipant = () =>
    dispatch<GroupViewInviteParticipantAction>({ type: GroupViewPluginActionType.InviteParticipant });
  const onSubmit = () => {
    dispatch<GroupViewSubmitAction>({ type: GroupViewPluginActionType.Submit });
    props.onClose();
  };

  return isNew ? (
    <NewGroupView
      noNativeFileBrowser
      groupName={groupName}
      onBack={props.onClose}
      canCreate
      onChangeGroupName={onChangeGroupName}
      onChooseImage={onChangeAvatar}
      onCreate={onSubmit}
      onInviteParticipant={onInviteParticipant}
      participants={participants}
      profilePicture={avatar ?? DefaultAvatar}
    />
  ) : (
    <EditGroupView
      noNativeFileBrowser
      groupName={groupName}
      onBack={props.onClose}
      onChangeGroupName={onChangeGroupName}
      onChooseImage={onChangeAvatar}
      onInviteParticipant={onInviteParticipant}
      participants={participants}
      profilePicture={avatar ?? DefaultAvatar}
      canSave
      onSave={onSubmit}
    />
  );
}
