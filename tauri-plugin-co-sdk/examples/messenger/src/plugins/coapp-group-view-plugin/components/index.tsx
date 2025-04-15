import { EditGroupView, NewGroupView } from "@1io/coapp-chatlist-view";
import React from "react";
import { useDispatch, useSelector } from "react-redux";
import DefaultAvatar from "../../../assets/Users_48.svg";
import { GroupViewLoadProfilePicAction, GroupViewPluginActionType, GroupViewSetNameAction } from "../actions/index.js";
import { GroupViewPluginState } from "../state/index.js";

export interface GroupViewContainerProps {
    readonly onClose: () => void;
}

export function GroupViewContainer(props: GroupViewContainerProps) {
    const dispatch = useDispatch();
    const groupName = useSelector((state: GroupViewPluginState) => state.name);
    const avatar = useSelector((state: GroupViewPluginState) => state.avatar);
    const isNew = useSelector((state: GroupViewPluginState) => state.isNew);
    const onChangeGroupName = (name: string) => {
        dispatch<GroupViewSetNameAction>({ payload: { name }, type: GroupViewPluginActionType.SetName });
    };
    const onChangeAvatar = () => dispatch<GroupViewLoadProfilePicAction>({ type: GroupViewPluginActionType.LoadProfilePicEpic });
    return isNew
        ? <NewGroupView
            noNativeFileBrowser
            groupName={groupName}
            onBack={props.onClose}
            canCreate
            onChangeGroupName={onChangeGroupName}
            onChooseImage={onChangeAvatar}
            onCreate={() => undefined}
            onInviteParticipant={() => undefined}
            participants={[]}
            profilePicture={avatar ?? DefaultAvatar}
        />
        : <EditGroupView
            noNativeFileBrowser
            groupName={groupName}
            onBack={props.onClose}
            onChangeGroupName={onChangeGroupName}
            onChooseImage={onChangeAvatar}
            onInviteParticipant={() => undefined}
            participants={[]}
            profilePicture={avatar ?? DefaultAvatar}
            canSave
            onSave={() => undefined}
        />;
}
