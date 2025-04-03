import { Chat } from "@1io/coapp-chatlist-view";
import { BaseApi, isPluginInitializeAction } from "@1io/kui-application-sdk";
import "@1io/packaging-utils/svg";
import { AnyAction } from "redux";
import { filter, identity, mergeAll, mergeMap } from "rxjs";
import { ChatsListActionType, ChatsListSetChatsAction } from "../actions/index.js";
import GroupDefaultPic from "../assets/Users_48.svg";
import { splitCoCoreId } from "../library/core-id.js";
import { invokeGetCoreState, invokeGetFilteredCores } from "../library/invoke-get.js";
import { ChatsListEpicType } from "../types/plugin.js";

export const initEpic: ChatsListEpicType = (action$, state$, context) => action$.pipe(
    filter(isPluginInitializeAction),
    mergeMap(async () => {
        const actions: AnyAction[] = [];
        // register plugin as base plugin with kui application
        const baseApi = context.api.getApi<BaseApi>([{ key: "type", value: "base" }]);
        actions.push(baseApi.set(
            context.plugin,
            [
                { key: "coapp-chats-list", value: context.plugin },
            ],
        ));
        // load all chat states
        const chats: Chat[] = [];
        const coreIds = await invokeGetFilteredCores(["core", "co-core-room"]);
        for (const coreId of coreIds) {
            // Split Id to co and core and cancel if failed
            const coCoreResult = splitCoCoreId(coreId);
            if (!coCoreResult) { continue }

            // get core state and cancel if failed
            const coreState = await invokeGetCoreState(coCoreResult.coId, coCoreResult.coreId);
            if (!coreState) { continue }

            // add to chats
            chats.push({
                name: coreState.name ?? "New room",
                id: coreId,
                newMessages: 0, // TODO check read receipts to know message count since last read
                avatar: GroupDefaultPic, // TODO use pic from CORE
            });
        }
        actions.push(identity<ChatsListSetChatsAction>({
            payload: { chats },
            type: ChatsListActionType.SetChats
        }));
        return actions;
    }),
    mergeAll(),
);
