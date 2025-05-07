import { Chat } from "@1io/coapp-chatlist-view";
import { isPluginInitializeAction } from "@1io/kui-application-sdk";
import { filter, identity, mergeAll, mergeMap } from "rxjs";
import { createIdentity, resolveCid, sessionClose, sessionOpen } from "../../../../../../dist-js/index.js";
import GroupDefaultPic from "../../../assets/Users_48.svg";
import { splitCoCoreId } from "../../../library/core-id.js";
import { getCoreState, getFilteredCoreIds } from "../../../library/invoke-get.js";
import { ChatsListActions, ChatsListActionType, ChatsListSetChatsAction, ChatsListSetIdentityAction } from "../actions/index.js";
import { ChatsListEpicType } from "../types/plugin.js";

const LOAD_IDENTITY_MAX_TRIES = 10;
const IDENTITY_NAME = "coapp_messenger";

export const loadChatsEpic: ChatsListEpicType = (action$) => action$.pipe(
    filter(isPluginInitializeAction),
    mergeMap(async () => {
        const actions: ChatsListActions[] = [];
        // create co application session
        let sessionId = await sessionOpen("local");

        // check identity
        let messengerIdentity: string | undefined = undefined;
        let tries = 0;
        do {
            // get identity
            messengerIdentity = await getCoappMessengerIdentity(sessionId);
            if (messengerIdentity === undefined) {
                // try to create identity
                await createIdentity(IDENTITY_NAME);
                tries++;
            }
        } while (messengerIdentity === undefined && tries < LOAD_IDENTITY_MAX_TRIES);

        // close active session to free up memory
        await sessionClose(sessionId);

        if (messengerIdentity === undefined) {
            // could not resolve identity error
            throw "Identity error: couldn't resolve messenger identity";
        }

        // save identity to state action
        actions.push(identity<ChatsListSetIdentityAction>({
            payload: { identity: messengerIdentity },
            type: ChatsListActionType.SetIdentity,
        }));

        // load all chat states
        const chats: Chat[] = [];
        const coreIds = await getFilteredCoreIds(["core", "co-core-room"]);
        for (const coreId of coreIds) {
            // Split Id to co and core and cancel if failed
            const coCoreResult = splitCoCoreId(coreId);
            if (!coCoreResult) { continue }

            // get core state and cancel if failed
            const coreState = await getCoreState(coCoreResult.coId, coCoreResult.coreId);
            if (!coreState) { continue }

            // add to chats
            chats.push({
                name: coreState.name ?? "New room",
                id: coreId,
                newMessages: 0, // TODO check read receipts to know message count since last read
                avatar: GroupDefaultPic, // TODO use pic from CORE
            });
        }
        actions.push(identity<ChatsListSetChatsAction>({
            payload: { chats },
            type: ChatsListActionType.SetChats
        }));
        return actions;
    }),
    mergeAll(),
);

/**
 * tries to resolve the IDENTITY_NAME identity from the CO kit
 */
async function getCoappMessengerIdentity(sessionId: string): Promise<undefined | string> {
    let keystoreState = await getCoreState("local", "keystore", sessionId);
    if (keystoreState === undefined || keystoreState === null) {
        return undefined;
    }
    let keyStoreKeys = await resolveCid(sessionId, keystoreState.keys);
    let messengerIdentity = keyStoreKeys.l.find((i: any) => i[1].name === IDENTITY_NAME);
    return messengerIdentity[0];
}

