import { BaseApi, isPluginInitializeAction } from "@1io/kui-application-sdk";
import "@1io/packaging-utils/svg";
import { filter, mergeMap } from "rxjs";
import { ChatsListEpicType } from "../types/plugin.js";

export const initEpic: ChatsListEpicType = (action$, state$, context) => action$.pipe(
    filter(isPluginInitializeAction),
    mergeMap(async () => {
        // register plugin as base plugin with kui application
        const baseApi = context.api.getApi<BaseApi>([{ key: "type", value: "base" }]);
        return baseApi.set(
            context.plugin,
            [
                { key: "coapp-chats-list", value: context.plugin },
            ],
        );
    }),
);
