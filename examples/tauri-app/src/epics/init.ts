import { BaseApi, isPluginInitializeAction } from "@1io/kui-application-sdk";
import { AnyAction } from "redux";
import { filter, mergeAll, mergeMap } from "rxjs";
import { ChatsListEpicType } from "../types/plugin";

export const initEpic: ChatsListEpicType = (action$, state$, context) => action$.pipe(
    filter(isPluginInitializeAction),
    mergeMap(async () => {
        const actions: AnyAction[] = [];
        const baseApi = context.api.getApi<BaseApi>([{ key: "type", value: "base" }]);
        actions.push(baseApi.set(
            context.plugin,
            [
                { key: "coapp-messenger", value: context.plugin },
            ],
        ));
        // TODO get all chats
        return actions;
    }),
    mergeAll(),
);
