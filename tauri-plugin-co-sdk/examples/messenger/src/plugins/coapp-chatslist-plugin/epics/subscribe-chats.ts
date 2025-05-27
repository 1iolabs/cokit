import { isPluginInitializeAction } from "@1io/kui-application-sdk";
import { Action } from "redux";
import { EMPTY, filter, identity, mergeAll, mergeMap, withLatestFrom } from "rxjs";
import { get_actions, resolveCid, sessionClose, sessionOpen } from "../../../../../../dist-js/index.js";
import GroupDefaultPic from "../../../assets/Users_48.svg";
import { createCoSdkStateEventListener } from "../../../library/co-sdk-state-listener.js";
import { buildCoCoreId } from "../../../library/core-id.js";
import { ChatsListActionType, ChatsListAddChatAction, ChatsListUpdateChatAction } from "../actions/index.js";
import { ChatsListEpicType } from "../types/plugin.js";

export const subscribeChatsEpic: ChatsListEpicType = (action$, state$, context) => action$.pipe(
    filter(isPluginInitializeAction),
    mergeMap(() => {
        return createCoSdkStateEventListener().pipe(
            // filter out events for the local co
            // filter((event) => { const [coId] = event.payload; return coId !== "local" }),
            withLatestFrom(state$),
            mergeMap(async ([event, state]) => {
                const [coId, _, heads] = event.payload;
                let sessionId = await sessionOpen(coId);
                const log = (await get_actions(sessionId, heads, 1, undefined)).actions;
                const actions: Action[] = [];
                for (const cid of log) {
                    const action = await resolveCid(sessionId, cid);
                    const payload = action.p;
                    console.log(coId, action);
                    if (coId === "local") {
                        return EMPTY;
                    }

                    // create core action
                    if (payload.CoreCreate !== undefined) {
                        actions.push(identity<ChatsListAddChatAction>({
                            payload: {
                                chat: {
                                    avatar: GroupDefaultPic,
                                    id: buildCoCoreId(coId, payload.CoreCreate.core),
                                    name: "",
                                    newMessages: 0,
                                }
                            },
                            type: ChatsListActionType.AddChat,
                        }));
                    }

                    // matrix events
                    switch (payload.type) {
                        case "m_room_message": {
                            const chat = state.chats.find((c) => c.id === buildCoCoreId(coId, action.c));
                            if (!chat) { continue }
                            actions.push(identity<ChatsListUpdateChatAction>({
                                payload: {
                                    chat: {
                                        lastMessage: {
                                            message: payload.content.body,
                                            key: payload.content.body,
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
                            let name = payload.content.name;
                            if (name) {
                                actions.push(identity<ChatsListUpdateChatAction>({
                                    payload: {
                                        chat: {
                                            id: buildCoCoreId(coId, action.c),
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
