import { Chat } from "@1io/coapp-chatlist-view";
import { isPluginInitializeAction } from "@1io/kui-application-sdk";
import { filter, identity, mergeAll, mergeMap } from "rxjs";
import { createIdentity } from "../../../../dist-js/index.js";
import { ChatsListActions, ChatsListActionType, ChatsListSetChatsAction } from "../actions/index.js";
import GroupDefaultPic from "../assets/Users_48.svg";
import { splitCoCoreId } from "../library/core-id.js";
import { invokeGetCoreState, invokeGetFilteredCores, invokeResolveCid } from "../library/invoke-get.js";
import { ChatsListEpicType } from "../types/plugin.js";

export const loadChatsEpic: ChatsListEpicType = (action$) => action$.pipe(
    filter(isPluginInitializeAction),
    mergeMap(async () => {
        const actions: ChatsListActions[] = [];
        // check identity
        let messengerIdentity: object | undefined = undefined;
        let tries = 0;
        do {
            messengerIdentity = await getCoappMessengerIdentity();
            if (messengerIdentity === undefined) {
                await createIdentity("coapp_messenger");
                tries++;
            }
        } while (messengerIdentity === undefined && tries < 3)
        console.log("keystore", messengerIdentity);

        // load all chat states
        const chats: Chat[] = [];
        const coreIds = await invokeGetFilteredCores(["core", "co-core-room"]);
        for (const coreId of coreIds) {
            // Split Id to co and core and cancel if failed
            const coCoreResult = splitCoCoreId(coreId);
            if (!coCoreResult) { continue }

            // get core state and cancel if failed
            const coreState = await invokeGetCoreState(coCoreResult.coId, coCoreResult.coreId);
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

async function getCoappMessengerIdentity(): Promise<undefined | object> {
    let keystoreState = await invokeGetCoreState("local", "keystore");
    if (keystoreState === undefined || keystoreState === null) {
        return undefined;
    }
    let keyStoreKeys = await invokeResolveCid("local", keystoreState.keys);
    let messengerIdentity = keyStoreKeys.l.find((i: any) => i[1].name === "coapp_messenger");
    return messengerIdentity;
}

