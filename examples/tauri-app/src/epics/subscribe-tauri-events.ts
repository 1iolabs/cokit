import { EMPTY, filter, mergeAll, mergeMap } from "rxjs";
import { MessengerEpicType } from "../types/plugin";
import { isPluginInitializeAction } from "@1io/kui-application-sdk";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { MessageReceivedAction, MessengerActionType } from "../actions";

export const subscribeTauriEventEpic: MessengerEpicType = (action$, _, context) => action$.pipe(
    filter(isPluginInitializeAction),
    mergeMap(async () => {
        console.log("listen");
        invoke("subscribe", { co: "1io" });
        const unlisten = await listen('test', (event) => {
            console.log("AYYYYY lmao", event);

            context.dispatch<MessageReceivedAction>({
                payload: { message: { key: "gref", message: "gre", ownMessage: true, timestamp: new Date } },
                type: MessengerActionType.MessageReceived,
            });
        });

        console.log("listen start", unlisten);
        return EMPTY;
    }),
    mergeAll(),
);