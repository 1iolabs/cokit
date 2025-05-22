import { Message, MessengerView } from "@1io/coapp-messenger-view";
import { usePluginActionApi, WellKnownTags } from "@1io/kui-application-sdk";
import "@1io/packaging-utils/svg";
import { CID } from "multiformats";
import * as React from "react";
import { useDispatch, useSelector } from "react-redux";
import { identity } from "rxjs";
import { resolveCid } from "../../../../../../dist-js/index.js";
import DefaultProfilePic from "../../../assets/Users_24.svg";
import { buildCoCoreId } from "../../../library/core-id.js";
import { COAppChatsListApi } from "../../coapp-chatslist-plugin/api/index.js";
import { coappChatsListPluginId } from "../../coapp-chatslist-plugin/types/plugin.js";
import { MessengerViewActionType, MessengerViewLoadMoreEventsAction, MessengerViewSendAction } from "../actions/index.js";
import { resolveMatrixAction } from "../library/handle-matrix-event.js";
import { MessengerViewPluginState } from "../types/state.js";


export interface MessengerViewContainerProps {
  onBack: () => void;
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
      for (let cid of cids) {
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
    }
  }, [cids, sessionId]);
  return ipldMap;
}

export function MessengerViewContainer(props: MessengerViewContainerProps) {

  const dispatch = useDispatch();
  const [message, setMessage] = React.useState("");
  const messageCids = useSelector((state: MessengerViewPluginState) => state.messages);
  const chatName = useSelector((state: MessengerViewPluginState) => state.chatName);
  const sessionId = useSelector((state: MessengerViewPluginState) => state.coSessionId);
  const ownIdentity = useSelector((state: MessengerViewPluginState) => state.chatsListState?.identity);
  const coCoreId = useSelector((state: MessengerViewPluginState) => buildCoCoreId(state.co, state.core));

  const api = usePluginActionApi<COAppChatsListApi>([{ key: WellKnownTags.Type, value: coappChatsListPluginId }]);

  const messageMap = useIpld<Message>(messageCids, resolveMatrixAction, sessionId, ownIdentity);
  const messages = React.useMemo(() => {
    const retVal: Message[] = [];
    for (let v of messageMap.values()) {
      if (v) {
        retVal.push(v);
      }
    }
    return retVal;
  }, [messageMap]);

  const onSendMessage = () => {
    // don't send empty messages
    if (message !== "") {
      dispatch<MessengerViewSendAction>({
        type: MessengerViewActionType.Send,
        payload: { message }
      });
      setMessage("");
    }
  }

  const onScrollTop = () => {
    dispatch(identity<MessengerViewLoadMoreEventsAction>({
      payload: { count: 30 },
      type: MessengerViewActionType.LoadMoreEvents,
    }));
  }

  const onInfo = () => {
    dispatch(api?.openGroupView(coCoreId));
  }

  return <MessengerView
    tauriWindowDragHeader
    chatInput={message}
    chatName={chatName}
    onChatInput={setMessage}
    messages={messages}
    onSendMessage={onSendMessage}
    onBack={props.onBack}
    onScrollTop={onScrollTop}
    onInfo={onInfo}
    profilePicture={DefaultProfilePic}
  />;
}
