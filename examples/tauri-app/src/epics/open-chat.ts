import { ApplicationApi, WellKnownTags } from "@1io/kui-application-sdk";
import { EMPTY, filter, mergeMap } from "rxjs";
import { ChatsListActionType } from "../actions";
import { ChatsListEpicType } from "../types/plugin";

export const openChatEpic: ChatsListEpicType = (action$, _, context) => action$.pipe(
    filter((action) => action.type === ChatsListActionType.OpenChat),
    mergeMap((action) => {
        const applicationApi = context.api.getApi<ApplicationApi>([{ key: WellKnownTags.Type, value: "application" }]);
        applicationApi.loadPlugin("coapp-messenger-view", []);
        // TODO load messenger view plugin with clicked on chat room
        return EMPTY;
    }),
);