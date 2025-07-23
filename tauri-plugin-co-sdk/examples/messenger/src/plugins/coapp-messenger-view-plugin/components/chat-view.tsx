import { Message, MessengerView } from "@1io/coapp-messenger-view";
import { usePluginActionApi, WellKnownTags } from "@1io/kui-application-sdk";
import "@1io/packaging-utils/svg";
import { CID } from "multiformats/cid";
import * as React from "react";
import { useDispatch, useSelector } from "react-redux";
import { fromEventPattern } from "rxjs";
import {
  getActions,
  GetActionsResponse,
  getCoState,
  resolveCid,
  sessionClose,
  sessionOpen,
  storageSet,
} from "../../../../../../dist-js/index.js";
import DefaultProfilePic from "../../../assets/Users_24.svg";
import { createCoSdkStateEventListener } from "../../../library/co-sdk-state-listener.js";
import { buildCoCoreId } from "../../../library/core-id.js";
import { COAppChatsListApi } from "../../coapp-chatslist-plugin/api/index.js";
import { coappChatsListPluginId } from "../../coapp-chatslist-plugin/types/plugin.js";
import { MessengerViewActionType, MessengerViewSendAction } from "../actions/index.js";
import { resolveMatrixAction } from "../library/handle-matrix-event.js";
import { MessengerViewPluginState } from "../types/state.js";

// the number of additional actions that should be loaded when scrolling to the top of the page
const MESSAGE_PAGING_COUNT = 30;

export interface MessengerViewContainerProps {
  onBack: () => void;
}

function useCoState(co: string): [CID | undefined, CID[] | undefined] {
  const [coState, setCoState] = React.useState<[CID | undefined, CID[]]>();

  React.useEffect(() => {
    // get initial state
    async function loadInitState() {
      const initState = await getCoState(co);
      setCoState(initState);
    }
    loadInitState();

    // get updated state when new event is triggered
    const coSdkEventSubscription = createCoSdkStateEventListener().subscribe({
      next: (event) => {
        const [coId, state, heads] = event.payload;
        if (co === coId) {
          setCoState([state, heads]);
        }
      },
    });
    return () => {
      coSdkEventSubscription.unsubscribe();
    };
  }, [co]);
  if (coState !== undefined) {
    return coState;
  }
  return [undefined, undefined];
}

function useCoreState(coCid: CID | undefined, coreId: string, session: string | undefined): any {
  const [coreState, setCoreState] = React.useState<any>(undefined);
  React.useEffect(() => {
    async function resolveCoreState() {
      if (session !== undefined && coCid !== undefined) {
        const resolvedCoState = await resolveCid(session, coCid);
        const coreCid = resolvedCoState?.cores?.[coreId]?.state;
        if (coreCid !== undefined) {
          const state = await resolveCid(session, coreCid);
          setCoreState(state);
        }

        const randomData = Uint8Array.from([5]);
        const response = await storageSet(session, randomData);
        console.log("cid dunno kp", response);
      }
    }
    resolveCoreState();
  }, [coCid, coreId, session]);
  return coreState;
}

function useSession(co: string): string | undefined {
  const [session, setSession] = React.useState<string | undefined>();
  const [sessionError, setSessionError] = React.useState<Error | undefined>(undefined);
  React.useEffect(() => {
    const sessionSubscription = fromEventPattern<string>(
      async (handler) => {
        const s = await sessionOpen(co);
        handler(s);
        return s;
      },
      async (_, sessionPromise) => {
        const s = await sessionPromise;
        console.log("unsub sesh", s);
        await sessionClose(s);
      },
    ).subscribe({
      next: (sessionEvent) => {
        setSession(sessionEvent);
      },
      error(err) {
        setSessionError(err);
      },
    });

    return () => {
      sessionSubscription.unsubscribe();
    };
  }, [co]);
  if (sessionError !== undefined) {
    throw sessionError;
  }
  return session;
}

function useCoCoreActions(
  co: string,
  core: string,
  heads: CID[] | undefined,
  session: string | undefined,
  count: number,
) {
  const [actions, setActions] = React.useState<GetActionsResponse>();
  React.useEffect(() => {
    async function getCoreActions() {
      if (heads !== undefined && session !== undefined) {
        setActions(await getActions(session, heads, count, undefined));
      }
    }
    getCoreActions();
  }, [co, core, heads, session, count]);
  return actions;
}

function useIpld<T>(
  cids: CID[],
  deserialize: (v: any, ownIdentity: string) => T | undefined,
  sessionId?: string,
  ownIdentity?: string,
): ReadonlyMap<CID, T | undefined> {
  const [ipldMap, setIpldMap] = React.useState<Map<CID, T | undefined>>(new Map());
  React.useEffect(() => {
    // cancel flag because component may unmount before fetch is done after which state changes become illegal
    let canceled = false;
    // async function that fetches the messages
    const fetchCids = async () => {
      if (sessionId === undefined || ownIdentity === undefined) {
        return;
      }
      const newMap = new Map();
      for (const cid of cids) {
        if (ipldMap.has(cid)) {
          // use value from old map
          newMap.set(cid, ipldMap.get(cid));
        } else {
          // fetch cid if not already loaded
          const ipld = await resolveCid(sessionId, cid);
          newMap.set(cid, deserialize(ipld, ownIdentity));
        }
      }
      // update map if component is still mounted
      if (!canceled) {
        setIpldMap(newMap);
      }
    };
    // call async fetch function
    fetchCids();
    // return deconstructor to cancel ongoing operations
    return () => {
      canceled = true;
    };
  }, [cids, sessionId]);
  return ipldMap;
}

export function MessengerViewContainer(props: MessengerViewContainerProps) {
  const dispatch = useDispatch();
  const ownIdentity = useSelector((state: MessengerViewPluginState) => state.chatsListState?.identity);
  const coId = useSelector((state: MessengerViewPluginState) => state.coId);
  const coreId = useSelector((state: MessengerViewPluginState) => state.coreId);

  const chatsListApi = usePluginActionApi<COAppChatsListApi>([
    { key: WellKnownTags.Type, value: coappChatsListPluginId },
  ]);

  // get co state
  const [coStateCid, coHeads] = useCoState(coId);
  console.log("co state cid", coStateCid);

  const session = useSession(coId);

  // get room core state
  const roomCoreState = useCoreState(coStateCid, coreId, session);
  console.log("core state", roomCoreState);

  // how many actions should be loaded
  const [actionCount, setActionCount] = React.useState(0);

  // get actions from co log
  const actions = useCoCoreActions(coId, coreId, coHeads, session, actionCount)?.actions;

  // get and resolve messages
  const messageMap = useIpld<Message>(actions ?? [], resolveMatrixAction, session, ownIdentity);
  const messages = React.useMemo(() => {
    const retVal: Message[] = [];
    for (const v of messageMap.values()) {
      if (v !== undefined) {
        retVal.push(v);
      }
    }
    // actions are in reverse chronological order, so newest items are in first index,
    // but we want the newest at the bottom
    return retVal.reverse();
  }, [messageMap]);

  // currently typed message
  const [message, setMessage] = React.useState("");

  // send message handler
  const onSendMessage = React.useCallback(() => {
    // don't send empty messages
    if (message !== "") {
      dispatch<MessengerViewSendAction>({
        payload: { message },
        type: MessengerViewActionType.Send,
      });
      setMessage("");
    }
  }, [message]);

  // load more actions handler
  const onScrollTop = React.useCallback(() => {
    setActionCount((c) => c + MESSAGE_PAGING_COUNT);
  }, []);

  // open group view
  const onInfo = React.useCallback(() => {
    const coCoreId = buildCoCoreId(coId, coreId);
    dispatch(chatsListApi?.openGroupView(coCoreId));
  }, [chatsListApi]);

  return (
    <MessengerView
      tauriWindowDragHeader
      chatInput={message}
      chatName={roomCoreState?.name ?? "Unnamed group"}
      onChatInput={setMessage}
      messages={messages}
      onSendMessage={onSendMessage}
      onBack={props.onBack}
      onScrollTop={onScrollTop}
      onInfo={onInfo}
      profilePicture={DefaultProfilePic}
    />
  );
}
