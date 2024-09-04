import { isPluginInitializeAction } from "@1io/kui-application-sdk";
import { EMPTY, filter, identity, mergeMap, withLatestFrom } from "rxjs";
import { createTauriSubscription } from "../../../library/create-tauri-subscribe.js";
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
        return createTauriSubscription<MessageEvent>(context.plugin, co, core).pipe(
            withLatestFrom(state$),
            // dedupe messages
            filter(([event, s]) => s.messages.findIndex((m) => m.key === event.payload.p.event_id) === -1),
            mergeMap(([event]) => {
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
