import { isPluginInitializeAction } from "@1io/kui-application-sdk";
import { EMPTY, filter, mergeMap, withLatestFrom } from "rxjs";
import { createCoSdkStateEventListener } from "../../../library/co-sdk-state-listener.js";
import { MessengerViewEpicType } from "../types/plugin.js";

export const subscribeTauriEventEpic: MessengerViewEpicType = (action$, state$, context) => action$.pipe(
    filter(isPluginInitializeAction),
    withLatestFrom(state$),
    mergeMap(([, state]) => {
        return createCoSdkStateEventListener().pipe(
            withLatestFrom(state$),
            // dedupe messages
            // filter(([event, s]) => s.messages.findIndex((m) => m.key === event.payload.p.event_id) === -1),
            mergeMap(([event]) => {
                // TODO
                /** 
                switch (event.payload.p.type) {
                    case "m_room_message": {
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
                    case "State": {
                        if (event.payload.p.content.type === "room_name") {
                            let groupName = event.payload.p.content.content.name;
                            if (groupName) {

                                return [identity<MessengerViewNameChangedAction>({
                                    payload: { newName: groupName },
                                    type: MessengerViewActionType.NameChanged,
                                })];
                            }
                        }
                    }
                }
                 */
                // event not handled
                return EMPTY;
            }),
        );
    }),
);
