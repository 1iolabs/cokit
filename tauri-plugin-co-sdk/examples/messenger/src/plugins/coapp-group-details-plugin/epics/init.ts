import { createPluginErroredAction, isPluginInitializeAction, tagValue } from "@1io/kui-application-sdk";
import { filter, identity, mergeAll, mergeMap } from "rxjs";
import DefaultAvatar from "../../../assets/Users_48.svg";
import { splitCoCoreId } from "../../../library/core-id.js";
import { invokeGetCoreState } from "../../../library/invoke-get.js";
import { GroupViewPluginActionType, GroupViewSetAvatarAction, GroupViewSetNameAction } from "../actions/index.js";
import { GroupViewEpicType } from "../types/plugin.js";
import { GroupViewPluginRoomCoreIdTag } from "../types/tag.js";

export const initializeEpic: GroupViewEpicType = (action$, _, context) => action$.pipe(
    filter(isPluginInitializeAction),
    mergeMap(async (action) => {
        const roomCoreId = tagValue<GroupViewPluginRoomCoreIdTag>(context.pluginTags, "roomCoreId");
        // edit mode => load current data
        if (roomCoreId !== undefined) {
            const result = splitCoCoreId(roomCoreId);
            if (result === undefined) {
                // Error!
                return [createPluginErroredAction({
                    name: "Invalid CoCOreId error",
                    message: "Couldn't resolve ids from CoCoreId",
                }, context.plugin, context.pluginTags)];
            }
            // get core state and cancel if failed
            const coreState = await invokeGetCoreState(result.coId, result.coreId);
            if (!coreState) {
                // Error!
                return [createPluginErroredAction({
                    name: "Invalid Core state error",
                    message: "Couldn't fetch core state",
                }, context.plugin, context.pluginTags)];
            }
            // TODO set information
            return [
                identity<GroupViewSetNameAction>({
                    payload: { name: coreState.name },
                    type: GroupViewPluginActionType.SetName,
                }),
                // TODO avatar
                identity<GroupViewSetAvatarAction>({
                    payload: { avatar: DefaultAvatar },
                    type: GroupViewPluginActionType.SetAvatar,
                }),
            ];
        } else {
            // TODO new mode
        }
        return [];
    }),
    mergeAll(),
);
