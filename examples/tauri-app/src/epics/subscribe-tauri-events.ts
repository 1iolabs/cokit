import { filter, fromEventPattern, map, mergeMap, withLatestFrom } from "rxjs";
import { MessengerEpicType } from "../types/plugin";
import { isPluginInitializeAction } from "@1io/kui-application-sdk";
import { Event, listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { MessengerActionType } from "../actions";
import { event } from "@tauri-apps/api";

interface MessageEvent {
    f: String;
    c: String;
    t: number;
    p: {
        event_id: String;
        timestamp: number;
        room_id: String;
        type: String;
        content: {
            msgtype: String;
            body: String;
        };
    };
}

export const subscribeTauriEventEpic: MessengerEpicType = (action$, state$, context) => action$.pipe(
    filter(isPluginInitializeAction),
    mergeMap(() => {
        console.log("listen");
        invoke("subscribe", { co: "1io", core: "room", event: "messaging" });
        const obs = fromEventPattern<Event<MessageEvent>>(
            async (handler) => {
                return await listen("messaging", handler);
            },
            (_handler, unlisten) => { console.log("remove"); unlisten() },
        ).pipe(
            withLatestFrom(state$),
            // dedupe messages
            filter(([event, state]) => state.messages.findIndex((m) => m.key === event.payload.p.event_id) === -1),
            map(([event]) => {
                console.log(event);
                const a = {
                    payload: {
                        message: {
                            key: event.payload.p.event_id,
                            message: event.payload.p.content.body,
                            ownMessage: true,
                            timestamp: new Date(event.payload.t),
                        }
                    },
                    type: MessengerActionType.MessageReceived,
                };
                return a;
            }),
        );
        console.log("listen start");
        return obs;
    }),
);
