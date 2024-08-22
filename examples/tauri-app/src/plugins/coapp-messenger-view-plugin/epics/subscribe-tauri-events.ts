import { EMPTY, filter, fromEventPattern, identity, mergeMap, withLatestFrom } from "rxjs";
import { MessengerViewEpicType } from "../types/plugin";
import { isPluginInitializeAction } from "@1io/kui-application-sdk";
import { Event, listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { MessengerViewNameChangedAction, MessengerViewActionType } from "../actions";

interface MessageEvent {
    f: string;
    c: string;
    t: number;
    p: {
        event_id: string;
        timestamp: number;
        room_id: string;
        type: string;
        content: {
            msgtype: string;
            body: string;
            name: string;
        };
    };
}

export const subscribeTauriEventEpic: MessengerViewEpicType = (action$, state$, context) => action$.pipe(
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
            mergeMap(([event]) => {
                console.log(event);
                if (event.payload.c !== "room") {
                    // not a room event, skip
                    return EMPTY;
                }
                switch (event.payload.p.type) {
                    case "m.room.message": {
                        return [{
                            payload: {
                                message: {
                                    key: event.payload.p.event_id,
                                    message: event.payload.p.content.body,
                                    ownMessage: true,
                                    timestamp: new Date(event.payload.t),
                                }
                            },
                            type: MessengerViewActionType.MessageReceived,
                        }];
                    };
                    case "m.room.name": {
                        let groupName = event.payload.p.content.name;
                        if (groupName) {

                            return [identity<MessengerViewNameChangedAction>({
                                payload: { newName: groupName },
                                type: MessengerViewActionType.NameChanged,
                            })];
                        }
                    }
                }
                // event not handled
                return EMPTY;
            }),
        );
    }),
);
