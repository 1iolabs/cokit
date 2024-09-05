import { EMPTY, filter, from, identity, map, mergeAll, mergeMap, takeUntil } from "rxjs";
import { ChatsListActionType, ChatsListSetChatsAction, ChatsListUpdateChatAction } from "../actions";
import { createTauriSubscription } from "../library/create-tauri-subscribe";
import { RoomCoreEvent } from "../types/message-event";
import { ChatsListEpicType } from "../types/plugin";

export const subscribeChatsEpic: ChatsListEpicType = (action$, _, context) => action$.pipe(
    filter((action): action is ChatsListSetChatsAction => action.type === ChatsListActionType.SetChats),
    mergeMap((action) => {
        return from(action.payload.chats).pipe(
            map((chat) => {
                const observable = createTauriSubscription<RoomCoreEvent>(context.plugin, chat.roomCoreId);
                return observable.pipe(
                    // stop subscription if chat gets removed 
                    // TODO: add check for chat ID
                    takeUntil(action$.pipe(filter((action) => action.type === ChatsListActionType.RemoveChat))),
                    mergeMap((event) => {
                        switch (event.payload.p.type) {
                            case "m.room.message": {
                                return [identity<ChatsListUpdateChatAction>({
                                    payload: {
                                        chat: {
                                            lastMessage: event.payload.p.content.body,
                                            newMessages: chat.newMessages + 1,
                                        },
                                        roomCoreId: chat.roomCoreId,
                                    },
                                    type: ChatsListActionType.UpdateChat,
                                })];
                            };
                            case "m.room.name": {
                                let name = event.payload.p.content.name;
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
                        return EMPTY;
                    }),
                );
            }),
        );
    }),
    mergeAll(),
);
