import { filter, identity, mergeAll, mergeMap } from "rxjs";
import { ChatsListActionType, ChatsListOpenChatDetailsAction, ChatsListSetPriorityPlugin } from "../actions/index.js";
import { coappGroupViewPluginId, GroupViewPluginRoomCoreIdTag } from "../plugins/coapp-group-details-plugin/types/tag.js";
import { ChatsListEpicType } from "../types/plugin.js";

export const groupDetailsEpic: ChatsListEpicType = (action$, _, context) => action$.pipe(
    filter((action): action is ChatsListOpenChatDetailsAction => action.type === ChatsListActionType.OpenChatDetails),
    mergeMap(async (action) => {
        const tags = [];
        if (action.payload.coCoreId !== undefined) {
            tags.push(identity<GroupViewPluginRoomCoreIdTag>({
                key: "roomCoreId", value: action.payload.coCoreId,
            }));
        }
        const pluginInfo = await context.api.loadPlugin(coappGroupViewPluginId, tags);
        return [identity<ChatsListSetPriorityPlugin>({ payload: { pluginId: pluginInfo.id }, type: ChatsListActionType.SetPriorityPlugin })];
    }),
    mergeAll(),
);
