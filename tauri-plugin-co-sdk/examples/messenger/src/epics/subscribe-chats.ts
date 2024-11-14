import { isPluginInitializeAction } from "@1io/kui-application-sdk";
import { Action } from "redux";
import { filter, identity, mergeAll, mergeMap, withLatestFrom } from "rxjs";
import { get_actions } from "../../../../dist-js";
import { ChatsListActionType, ChatsListUpdateChatAction } from "../actions";
import { createCoSdkStateEventListener } from "../library/co-sdk-state-listener";
import { buildCoCoreId } from "../library/core-id";
import { invokeResolveCid } from "../library/invoke-get";
import { ChatsListEpicType } from "../types/plugin";

export const subscribeChatsEpic: ChatsListEpicType = (action$, state$, context) => action$.pipe(
    filter(isPluginInitializeAction),
    mergeMap(() => {
        return createCoSdkStateEventListener().pipe(
            filter((event) => { const [co] = event.payload; return co !== "local" }),
            withLatestFrom(state$),
            mergeMap(async ([event, state]) => {
                const [co, _, heads] = event.payload;

                const log = (await get_actions(co, heads, 1, undefined)).actions;
                const actions: Action[] = [];
                for (const cid of log) {
                    const payload = await invokeResolveCid(co, cid);
                    console.log("action", payload);

                    switch (payload.p.type) {
                        case "m_room_message": {
                            const chat = state.chats.find((c) => c.roomCoreId === buildCoCoreId(co, payload.c));
                            if (!chat) { continue }
                            actions.push(identity<ChatsListUpdateChatAction>({
                                payload: {
                                    chat: {
                                        lastMessage: payload.p.content.body,
                                        // don't tick up message count if chat is currently shown
                                        newMessages: chat?.pluginId === state.activePlugin
                                            ? 0
                                            : chat.newMessages + 1,
                                        roomCoreId: chat.roomCoreId,
                                    },
                                },
                                type: ChatsListActionType.UpdateChat,
                            }));
                            break;
                        };
                        case "State": {
                            if (payload.p.content.type === "room_name") {
                                let name = payload.p.content.content.name;
                                const chat = state.chats.find((c) => c.roomCoreId === buildCoCoreId(co, payload.c));
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
                }
                return actions;
            }),
            mergeAll(),
        );
    }),
);
