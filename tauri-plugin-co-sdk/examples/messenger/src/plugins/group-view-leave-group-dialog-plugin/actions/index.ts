import { PluginPublicAction } from "@1io/kui-application-sdk";

export enum LeaveGroupDialogActionType {
    Leave = "coapp/group-view/leave-group-dialog/leave",
}

export type LeaveGroupDialogActions = LeaveGroupDialogLeaveAction;

export type LeaveGroupDialogLeaveAction = PluginPublicAction<LeaveGroupDialogActionType.Leave>;
