import { isPluginInitializeAction } from "@1io/kui-application-sdk";
import { invoke } from "@tauri-apps/api/core";
import { Event, listen } from "@tauri-apps/api/event";
import { EMPTY, filter, fromEventPattern, identity, mergeMap, withLatestFrom } from "rxjs";
import { buildCoCoreId } from "../../../library/core-id.js";
import { MessengerViewActionType, MessengerViewNameChangedAction } from "../actions";
import { MessengerViewEpicType } from "../types/plugin";

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
    withLatestFrom(state$),
    mergeMap(([, state]) => {
        const co = state.co;
        const core = state.core;
        console.log("subscribe co/core", co, core);
        invoke("subscribe", { co, core, source: context.plugin });
        return fromEventPattern<Event<MessageEvent>>(
            async (handler) => {
                return await listen(buildCoCoreId(co, core), handler);
            },
            (_handler, unlisten) => {
                console.log("remove", context.plugin);
                invoke("unsubscribe", { co, core, source: context.plugin });
                unlisten();
            },
        ).pipe(
            withLatestFrom(state$),
            // dedupe messages
            filter(([event, state]) => state.messages.findIndex((m) => m.key === event.payload.p.event_id) === -1),
            mergeMap(([event]) => {
                console.log(event);
                if (event.payload.c !== core) {
                    // not an event of this core, skip
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
