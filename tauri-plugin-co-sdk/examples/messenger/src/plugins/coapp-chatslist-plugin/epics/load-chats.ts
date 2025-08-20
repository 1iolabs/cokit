import { isPluginInitializeAction } from "@1io/kui-application-sdk";
import { filter, identity, mergeAll, mergeMap } from "rxjs";
import { createIdentity, Keystore, resolveCid, sessionClose, sessionOpen } from "../../../../../../dist-js/index.js";
import GroupDefaultPic from "../../../assets/Users_48.svg";
import { splitCoCoreId } from "../../../library/core-id.js";
import { getCoreState, getFilteredCoreIds } from "../../../library/invoke-get.js";
import {
  ChatsListActions,
  ChatsListActionType,
  ChatsListAddChatAction,
  ChatsListSetIdentityAction,
} from "../actions/index.js";
import { ChatsListEpicType } from "../types/plugin.js";
import { DagList } from "../../../library/dag-list.js";

const LOAD_IDENTITY_MAX_TRIES = 2;
const IDENTITY_NAME = "coapp_messenger";

export const loadChatsEpic: ChatsListEpicType = (action$) =>
  action$.pipe(
    filter(isPluginInitializeAction),
    mergeMap(async () => {
      const actions: ChatsListActions[] = [];
      // create co application session
      const sessionId = await sessionOpen("local");

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
        throw new Error("Identity error: couldn't resolve messenger identity");
      }

      // save identity to state action
      actions.push(
        identity<ChatsListSetIdentityAction>({
          payload: { identity: messengerIdentity },
          type: ChatsListActionType.SetIdentity,
        }),
      );

      // load all chat states
      const coreIds = await getFilteredCoreIds(["core", "co-core-room"]);
      for (const coreId of coreIds) {
        // Split Id to co and core and cancel if failed
        const coCoreResult = splitCoCoreId(coreId);
        if (coCoreResult === undefined) {
          continue;
        }

        let coreState;
        // get core state and cancel if failed
        try {
          coreState = await getCoreState(coCoreResult.coId, coCoreResult.coreId);
        } catch (e) {
          console.error("Error while fetching state: ", e);
        }
        if (coreState === undefined) {
          continue;
        }

        // add to actions
        actions.push(
          identity<ChatsListAddChatAction>({
            payload: {
              chat: {
                avatar: GroupDefaultPic, // TODO use pic from CORE
                id: coreId,
                name: coreState.name ?? "New room",
                newMessages: 0, // TODO check read receipts to know message count since last read
              },
            },
            type: ChatsListActionType.AddChat,
          }),
        );
      }
      return actions;
    }),
    mergeAll(),
  );

/**
 * tries to resolve the IDENTITY_NAME identity from the CO kit
 */
async function getCoappMessengerIdentity(sessionId: string): Promise<undefined | string> {
  const keystoreState = await getCoreState("local", "keystore", sessionId);
  if (keystoreState?.keys === undefined || keystoreState === null) {
    return undefined;
  }
  const keyStoreKeys = await resolveCid(sessionId, keystoreState.keys);
  // TODO use wasm isntead
  // rust map is lsm tree so this might not always work
  const dagList = new DagList<[string, { v?: Keystore.Key; t?: undefined }]>(keyStoreKeys.a, sessionId);
  const messengerIdentity = await dagList.find((i) => i[1].v?.name === IDENTITY_NAME);
  return messengerIdentity?.[0];
}
