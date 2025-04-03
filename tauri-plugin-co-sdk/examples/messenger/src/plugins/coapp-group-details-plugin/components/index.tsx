import { EditGroupView, NewGroupView } from "@1io/coapp-chatlist-view";
import React from "react";
import { useSelector } from "react-redux";
import DefaultAvatar from "../../../assets/Users_48.svg";
import { GroupViewPluginState } from "../state/index.js";

export function GroupViewContainer() {
    const groupName = useSelector((state: GroupViewPluginState) => state.name);
    const avatar = useSelector((state: GroupViewPluginState) => state.avatar);
    const isNew = useSelector((state: GroupViewPluginState) => state.isNew);
    return isNew
        ? <NewGroupView
            groupName={groupName}
            onBack={() => undefined}
            canCreate
            onChangeGroupName={() => undefined}
            onChooseImage={() => undefined}
            onCreate={() => undefined}
            onInviteParticipant={() => undefined}
            participants={[]}
            profilePicture={avatar ?? DefaultAvatar}
        />
        : <EditGroupView
            groupName={groupName}
            onBack={() => undefined}
            onChangeGroupName={() => undefined}
            onChooseImage={() => undefined}
            onInviteParticipant={() => undefined}
            participants={[]}
            profilePicture={avatar ?? DefaultAvatar}
            canSave
            onSave={() => undefined}
        />;
}
