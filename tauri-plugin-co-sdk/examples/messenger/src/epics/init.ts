import { Chat } from "@1io/coapp-chatlist-view";
import { BaseApi, isPluginInitializeAction } from "@1io/kui-application-sdk";
import "@1io/packaging-utils/svg";
import { AnyAction } from "redux";
import { filter, identity, mergeAll, mergeMap } from "rxjs";
import { ChatsListActionType, ChatsListSetChatsAction } from "../actions/index.js";
import GroupDefaultPic from "../assets/Users.svg";
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
            const [co, core] = splitCoCoreId(coreId);
            if (core) {
                const coreState = await invokeGetCoreState(co, core);
                if (coreState) {
                    chats.push({
                        name: coreState.name ?? "New room",
                        id: coreId,
                        newMessages: 0,
                        avatar: GroupDefaultPic, // TODO use pic from CORE
                    });
                }
            }
        }
        console.log("cores", chats);
        actions.push(identity<ChatsListSetChatsAction>({
            payload: { chats },
            type: ChatsListActionType.SetChats
        }));
        return actions;
    }),
    mergeAll(),
);
