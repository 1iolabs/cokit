import { filter, fromEventPattern, map, mergeMap, withLatestFrom } from "rxjs";
import { MessengerEpicType } from "../types/plugin";
import { isPluginInitializeAction } from "@1io/kui-application-sdk";
import { Event, listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { MessengerActionType } from "../actions";

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
        console.log("subscribe 1io/room");
        invoke("subscribe", { co: "1io", core: "room", source: context.plugin });
        return fromEventPattern<Event<MessageEvent>>(
            async (handler) => {
                return await listen("1io/room", handler);
            },
            (_handler, unlisten) => {
                console.log("remove", context.plugin);
                invoke("unsubscribe", { co: "1io", core: "room", source: context.plugin });
                unlisten();
            },
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
    }),
);
