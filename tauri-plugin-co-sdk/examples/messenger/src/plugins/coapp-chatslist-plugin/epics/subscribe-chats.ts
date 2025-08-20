import { isPluginInitializeAction } from "@1io/kui-application-sdk";
import { Action } from "redux";
import { filter, identity, mergeAll, mergeMap, withLatestFrom } from "rxjs";
import { getActions, pushAction, resolveCid, sessionClose, sessionOpen } from "../../../../../../dist-js/index.js";
import GroupDefaultPic from "../../../assets/Users_48.svg";
import { createCoSdkStateEventListener } from "../../../library/co-sdk-state-listener.js";
import { buildCoCoreId, splitCoCoreId } from "../../../library/core-id.js";
import { getCoreState, getFilteredCoreIds } from "../../../library/invoke-get.js";
import { ChatsListActionType, ChatsListAddChatAction, ChatsListUpdateChatAction } from "../actions/index.js";
import { ChatsListEpicType } from "../types/plugin.js";

export const subscribeChatsEpic: ChatsListEpicType = (action$, state$, context) =>
  action$.pipe(
    filter(isPluginInitializeAction),
    mergeMap(() => {
      return createCoSdkStateEventListener().pipe(
        withLatestFrom(state$),
        mergeMap(async ([event, state]) => {
          const [coId, _, heads] = event;
          // if (coId === "local") { return EMPTY }
          console.log(coId, heads);
          const sessionId = await sessionOpen(coId);
          const log = (await getActions(sessionId, heads, 1, undefined)).actions;
          const actions: Action[] = [];
          for (const cid of log) {
            const action = await resolveCid(sessionId, cid);
            const payload = action.p;
            console.log("Action pushed: ", coId, action);
            if (coId === "local" && state.identity !== undefined) {
              actions.push(...(await handleLocalCoAction(action, state.identity)));
              continue;
            }

            // create core action
            if (payload.CoreCreate !== undefined) {
              actions.push(
                identity<ChatsListAddChatAction>({
                  payload: {
                    chat: {
                      avatar: GroupDefaultPic,
                      id: buildCoCoreId(coId, payload.CoreCreate.core),
                      name: "",
                      newMessages: 0,
                    },
                  },
                  type: ChatsListActionType.AddChat,
                }),
              );
            }

            // matrix events
            switch (payload.type) {
              case "m_room_message": {
                const chat = state.chats.find((c) => c.id === buildCoCoreId(coId, action.c));
                if (chat === undefined) {
                  continue;
                }
                actions.push(
                  identity<ChatsListUpdateChatAction>({
                    payload: {
                      chat: {
                        id: chat.id,
                        lastMessage: {
                          key: payload.content.body,
                          message: payload.content.body,
                          ownMessage: false,
                          timestamp: new Date(),
                        },
                        // don't tick up message count if chat is currently shown
                        newMessages: state.selectedChat === chat.id ? 0 : chat.newMessages + 1,
                      },
                    },
                    type: ChatsListActionType.UpdateChat,
                  }),
                );
                break;
              }
              case "room_name": {
                const name = payload.content.name;
                if (name) {
                  actions.push(
                    identity<ChatsListUpdateChatAction>({
                      payload: {
                        chat: {
                          id: buildCoCoreId(coId, action.c),
                          name,
                        },
                      },
                      type: ChatsListActionType.UpdateChat,
                    }),
                  );
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

async function handleLocalCoAction(action: any, ownIdentity: string): Promise<Action[]> {
  if (action.c === "membership") {
    if (action.p?.Join !== undefined) {
      // only continue if we are the invited did
      if (action.p.Join.did !== ownIdentity) {
        return [];
      }
      // only continue if the join action is on invite state
      if (action.p.Join.membership_state !== 2) {
        return [];
      }
      // accept join action
      const joinAction = {
        ChangeMembershipState: {
          did: ownIdentity,
          id: action.p.Join.id,
          membership_state: 3,
        },
      };
      console.log("accept action", joinAction);
      try {
        const session = await sessionOpen("local");
        await pushAction(session, "membership", joinAction, ownIdentity);
        await sessionClose(session);
      } catch (e) {
        console.log(e);
      }
    }
    if (action.p?.ChangeMembershipState !== undefined) {
      const changeMemberAction = action.p?.ChangeMembershipState;

      // only actions that added us are relevant
      if (changeMemberAction.did !== ownIdentity) {
        return [];
      }

      // only continue if join accepted
      if (changeMemberAction.membership_state !== 0) {
        return [];
      }

      // get all room cores of joined co
      const roomCoreIds = await getFilteredCoreIds(["core", "co-core-room"], changeMemberAction.id);

      // get chats
      const addChatActions: ChatsListAddChatAction[] = [];
      for (const roomCoreId of roomCoreIds) {
        const ids = splitCoCoreId(roomCoreId);
        if (ids === undefined) {
          continue;
        }
        const state = await getCoreState(ids.coId, ids.coreId);
        addChatActions.push(
          identity<ChatsListAddChatAction>({
            payload: {
              chat: {
                avatar: GroupDefaultPic,
                id: roomCoreId,
                name: state.name,
                newMessages: 0,
              },
            },
            type: ChatsListActionType.AddChat,
          }),
        );
      }
      return addChatActions;
    }
  }
  return [];
}
