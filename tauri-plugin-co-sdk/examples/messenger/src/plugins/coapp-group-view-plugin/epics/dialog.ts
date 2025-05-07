import { isPluginAction, WellKnownTags } from "@1io/kui-application-sdk";
import { filter, identity, map, mergeAll, mergeMap, withLatestFrom } from "rxjs";
import { COAppChatsListApi } from "../../coapp-chatslist-plugin/api/index.js";
import { coappChatsListPluginId } from "../../coapp-chatslist-plugin/types/plugin.js";
import { AddParticipantDialogActionType, AddParticipantDialogSaveAction } from "../../group-view-add-participant-dialog-plugin/actions/index.js";
import { addParticipantDialogPluginId } from "../../group-view-add-participant-dialog-plugin/types/plugin.js";
import { LeaveGroupDialogActionType, LeaveGroupDialogLeaveAction } from "../../group-view-leave-group-dialog-plugin/actions/index.js";
import { LeaveGroupDialogGroupNameTag, leaveGroupDialogPluginId } from "../../group-view-leave-group-dialog-plugin/types/tag.js";
import { RemoveParticipantDialogActionType, RemoveParticipantDialogRemoveAction } from "../../group-view-remove-participant-dialog-plugin/actions/index.js";
import { removeParticipantDialogPluginId, RemoveParticipantDialogRequiredTags } from "../../group-view-remove-participant-dialog-plugin/types/plugin.js";
import { GroupViewParticipantInvitedAction, GroupViewParticipantRemovedAction, GroupViewPluginActionType, GroupViewRemoveParticipantAction } from "../actions/index.js";
import { GroupViewEpicType } from "../types/plugin.js";


export const openInviteParticipantDialogEpic: GroupViewEpicType = (action$, _, context) => action$.pipe(
    filter((action) => action.type === GroupViewPluginActionType.InviteParticipant),
    mergeMap(async () => {
        const api = context.api.getApi<COAppChatsListApi>([{ key: WellKnownTags.Type, value: coappChatsListPluginId }]);
        const [loadAction, pluginId] = await api.loadDialog(addParticipantDialogPluginId, []);

        return [loadAction, context.api.subscribeActions(pluginId)];
    }),
    mergeAll(),
);

export const participantAddedEpic: GroupViewEpicType = (action$, state$, context) => action$.pipe(
    filter(isPluginAction),
    map((action) => {
        return action.payload;
    }),
    filter((action): action is AddParticipantDialogSaveAction => action.type === AddParticipantDialogActionType.Save),
    mergeMap((action) => {
        return [identity<GroupViewParticipantInvitedAction>({
            payload: { participant: action.payload.did },
            type: GroupViewPluginActionType.ParticipantInvited,
        })];
    }),
);

export const openRemoveParticipantDialogEpic: GroupViewEpicType = (action$, state$, context) => action$.pipe(
    filter((action): action is GroupViewRemoveParticipantAction => action.type === GroupViewPluginActionType.RemoveParticipant),
    withLatestFrom(state$),
    mergeMap(async ([action, state]) => {
        const api = context.api.getApi<COAppChatsListApi>([{ key: WellKnownTags.Type, value: coappChatsListPluginId }]);
        if (action.payload.isYou) {
            // leave group
            const groupNameTag: LeaveGroupDialogGroupNameTag = { key: "groupName", value: state.name };
            const [loadAction, pluginId] = await api.loadDialog(leaveGroupDialogPluginId, [groupNameTag]);

            return [loadAction, context.api.subscribeActions(pluginId)];
            return [];
        } else {
            // remove from group
            const removeParticipantDialogTags: RemoveParticipantDialogRequiredTags = [
                { key: "did", value: action.payload.participant },
                { key: "groupName", value: state.name },
            ];
            const [loadAction, pluginId] = await api.loadDialog(removeParticipantDialogPluginId, removeParticipantDialogTags);

            return [loadAction, context.api.subscribeActions(pluginId)];
        }
    }),
    mergeAll(),
);

export const leftGroupEpic: GroupViewEpicType = (action$, state$, context) => action$.pipe(
    filter(isPluginAction),
    map((action) => {
        return action.payload;
    }),
    filter((action): action is LeaveGroupDialogLeaveAction => action.type === LeaveGroupDialogActionType.Leave),
    withLatestFrom(state$),
    mergeMap(([action, state]) => {
        // TODO leave group
        // close group view
        // remove group from groups list
        // unsubscribe from room events
        return [];
    }),
);

export const participantRemovedEpic: GroupViewEpicType = (action$, state$, context) => action$.pipe(
    filter(isPluginAction),
    map((action) => {
        return action.payload;
    }),
    filter((action): action is RemoveParticipantDialogRemoveAction => action.type === RemoveParticipantDialogActionType.Remove),
    mergeMap((action) => {
        return [identity<GroupViewParticipantRemovedAction>({
            payload: { participant: action.payload.did },
            type: GroupViewPluginActionType.ParticipantRemoved,
        })];
    }),
);
