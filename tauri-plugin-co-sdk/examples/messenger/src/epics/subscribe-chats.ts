import { isPluginInitializeAction } from "@1io/kui-application-sdk";
import { EMPTY, filter, mergeMap, withLatestFrom } from "rxjs";
import { createCoSdkStateEventListener } from "../library/co-sdk-state-listener";
import { ChatsListEpicType } from "../types/plugin";

export const subscribeChatsEpicc: ChatsListEpicType = (action$, state$, context) => action$.pipe(
    filter(isPluginInitializeAction),
    mergeMap(() => {
        return createCoSdkStateEventListener().pipe(
            withLatestFrom(state$),
            mergeMap(([event, state]) => {
                const [co] = event.payload;
                console.log("co", co);
                // TODO poll latest actions from co

                // for all new actions: check if we need to update UI information:
                /** 
                switch (event.payload.p.type) {
                    case "m_room_message": {
                        const pluginId = state.chats.find((c) => c.roomCoreId === chat.roomCoreId)?.pluginId;
                        return [identity<ChatsListUpdateChatAction>({
                            payload: {
                                chat: {
                                    lastMessage: event.payload.p.content.body,
                                    // don't tick up message count if chat is currently shown
                                    newMessages: pluginId === state.activePlugin
                                    ? 0
                                    : chat.newMessages + 1,
                                },
                                roomCoreId: chat.roomCoreId,
                            },
                            type: ChatsListActionType.UpdateChat,
                        })];
                    };
                    case "State": {
                        if (event.payload.p.content.type === "room_name") {
                            let name = event.payload.p.content.content.name;
                            if (name) {
                                return [identity<ChatsListUpdateChatAction>({
                                    payload: {
                                        chat: {
                                            ...chat,
                                            name,
                                        },
                                        roomCoreId: chat.roomCoreId,
                                    },
                                    type: ChatsListActionType.UpdateChat,
                                })];
                            }
                        }
                    }
                }
                */

                return EMPTY;
            }),
        );
    }),
);
