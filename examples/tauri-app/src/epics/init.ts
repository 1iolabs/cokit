import { isPluginInitializeAction, BaseApi } from "@1io/kui-application-sdk";
import { filter, map } from "rxjs";
import { MessengerEpicType } from "../types/plugin";

export const initEpic: MessengerEpicType = (action$, state$, context) => action$.pipe(
    filter(isPluginInitializeAction),
    map(() => {
        const baseApi = context.api.getApi<BaseApi>([{ key: "type", value: "base" }]);
        return baseApi.set(
            context.plugin,
            [
                { key: "coapp-messenger", value: context.plugin },
            ],
        );
    }),

);
