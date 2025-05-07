import { LeaveGroupDialog } from "@1io/coapp-chatlist-view";
import React from "react";
import { useDispatch } from "react-redux";
import { DialogProps } from "../../../types/dialog-container-props.js";
import { LeaveGroupDialogActionType, LeaveGroupDialogLeaveAction } from "../actions/index.js";

export interface LeaveGroupDialogContainerProps extends DialogProps {
    readonly groupName: string;
}

export function LeaveGroupDialogContainer(props: LeaveGroupDialogContainerProps) {
    const dispatch = useDispatch();
    const onClickLeaveGroup = () => {
        dispatch<LeaveGroupDialogLeaveAction>({
            public: true,
            type: LeaveGroupDialogActionType.Leave,
        });
        props.onClose();
    };
    return <LeaveGroupDialog
        groupName={props.groupName}
        onBack={props.onClose}
        onClickLeaveGroup={onClickLeaveGroup}
    />;
}
