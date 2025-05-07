import { isPluginInitializeAction } from "@1io/kui-application-sdk";
import { Action } from "redux";
import { filter, identity, mergeAll, mergeMap, withLatestFrom } from "rxjs";
import { get_actions, resolveCid, sessionClose, sessionOpen } from "../../../../../../dist-js/index.js";
import { createCoSdkStateEventListener } from "../../../library/co-sdk-state-listener.js";
import { buildCoCoreId } from "../../../library/core-id.js";
import { ChatsListActionType, ChatsListUpdateChatAction } from "../actions/index.js";
import { ChatsListEpicType } from "../types/plugin.js";

export const subscribeChatsEpic: ChatsListEpicType = (action$, state$, context) => action$.pipe(
    filter(isPluginInitializeAction),
    mergeMap(() => {
        return createCoSdkStateEventListener().pipe(
            // filter out events for the local co
            filter((event) => { const [coId] = event.payload; return coId !== "local" }),
            withLatestFrom(state$),
            mergeMap(async ([event, state]) => {
                const [coId, _, heads] = event.payload;
                let sessionId = await sessionOpen(coId);
                const log = (await get_actions(sessionId, heads, 1, undefined)).actions;
                const actions: Action[] = [];
                for (const cid of log) {
                    const payload = await resolveCid(sessionId, cid);
                    const matrixEvent = payload.p;

                    switch (matrixEvent.type) {
                        case "m_room_message": {
                            const chat = state.chats.find((c) => c.id === buildCoCoreId(coId, payload.c));
                            if (!chat) { continue }
                            actions.push(identity<ChatsListUpdateChatAction>({
                                payload: {
                                    chat: {
                                        lastMessage: {
                                            message: matrixEvent.content.body,
                                            key: matrixEvent.content.body,
                                            ownMessage: false,
                                            timestamp: new Date(),
                                        },
                                        // don't tick up message count if chat is currently shown
                                        newMessages: state.selectedChat === chat.id
                                            ? 0
                                            : chat.newMessages + 1,
                                        id: chat.id,
                                    },
                                },
                                type: ChatsListActionType.UpdateChat,
                            }));
                            break;
                        };
                        case "room_name": {
                            let name = matrixEvent.content.name;
                            const chat = state.chats.find((c) => c.id === buildCoCoreId(coId, payload.c));
                            if (name && chat) {
                                actions.push(identity<ChatsListUpdateChatAction>({
                                    payload: {
                                        chat: {
                                            ...chat,
                                            name,
                                        },
                                    },
                                    type: ChatsListActionType.UpdateChat,
                                }));
                            }
                        }
                    }
                }
                await sessionClose(sessionId);
                return actions;
            }),
            mergeAll(),
        );
    }),
);
