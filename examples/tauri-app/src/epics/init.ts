import { BaseApi, isPluginInitializeAction } from "@1io/kui-application-sdk";
import { AnyAction } from "redux";
import { filter, identity, mergeAll, mergeMap } from "rxjs";
import { ChatsListActionType, ChatsListSetChatsAction } from "../actions";
import { splitCoCoreId } from "../library/core-id";
import { invokeGetCoreState, invokeGetFilteredCores } from "../library/invoke-get";
import { Chat } from "../state";
import { ChatsListEpicType } from "../types/plugin";

export const initEpic: ChatsListEpicType = (action$, state$, context) => action$.pipe(
    filter(isPluginInitializeAction),
    mergeMap(async () => {
        const actions: AnyAction[] = [];
        const baseApi = context.api.getApi<BaseApi>([{ key: "type", value: "base" }]);
        actions.push(baseApi.set(
            context.plugin,
            [
                { key: "coapp-chats-list", value: context.plugin },
            ],
        ));
        const chats: Chat[] = [];
        const coreIds = await invokeGetFilteredCores(["core", "co-core-room"]);
        for (const coreId of coreIds) {
            try {
                const [co, core] = splitCoCoreId(coreId);
                const coreState = await invokeGetCoreState(co, core);
                if (coreState?.name) {
                    chats.push({ name: coreState.name, roomCoreId: coreId });
                }
            } catch (err) {
                console.log(err);
            }
        }
        actions.push(identity<ChatsListSetChatsAction>({
            payload: { chats },
            type: ChatsListActionType.SetChats
        }));
        return actions;
    }),
    mergeAll(),
);
