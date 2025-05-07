import { isPluginInitializeAction, isPluginShutdownAction, WellKnownTags } from "@1io/kui-application-sdk";
import { AnyAction } from "redux";
import { EMPTY, filter, identity, map, mergeAll, mergeMap, take, withLatestFrom } from "rxjs";
import { sessionClose, sessionOpen } from "../../../../../../dist-js/index.js";
import { getCoreState } from "../../../library/invoke-get.js";
import { coappChatsListPluginId } from "../../coapp-chatslist-plugin/types/plugin.js";
import { MessengerViewActionType, MessengerViewLoadMoreEventsAction, MessengerViewNameChangedAction, MessengerViewSetSessionAction } from "../actions/index.js";
import { MessengerViewEpicType } from "../types/plugin.js";

export const initEpic: MessengerViewEpicType = (action$, state$, context) => action$.pipe(
    filter(isPluginInitializeAction),
    withLatestFrom(state$),
    mergeMap(async ([, state]) => {
        const actions: AnyAction[] = [];
        // open session 
        const sessionId = await sessionOpen(state.co);
        actions.push(identity<MessengerViewSetSessionAction>({
            payload: { sessionId },
            type: MessengerViewActionType.SetSession,
        }));

        // load core state of room
        const roomCoreState = await getCoreState(state.co, state.core, sessionId);
        const chatName = roomCoreState?.name;
        if (chatName) {
            actions.push(identity<MessengerViewNameChangedAction>({
                payload: { newName: chatName },
                type: MessengerViewActionType.NameChanged,
            }));
        }

        // load first messages
        actions.push(identity<MessengerViewLoadMoreEventsAction>({
            payload: { count: 30 },
            type: MessengerViewActionType.LoadMoreEvents,
        }));

        // subscribe chats list public state
        actions.push(context.api.subscribeState(
            [{ key: WellKnownTags.Type, value: coappChatsListPluginId }],
            "chatsListState",
        ));
        return actions;
    }),
    mergeAll(),
);

export const shutdownEpic: MessengerViewEpicType = (action$, state$, context) => action$.pipe(
    filter(isPluginShutdownAction),
    withLatestFrom(
        state$.pipe(
            filter((s) => s?.coSessionId !== undefined),
            map((s) => s.coSessionId),
            take(1),
        )
    ),
    mergeMap(async ([, session]) => {
        // close potential session before unload
        if (session) {
            await sessionClose(session);
        }
        return EMPTY;
    }),
    mergeAll(),
);
