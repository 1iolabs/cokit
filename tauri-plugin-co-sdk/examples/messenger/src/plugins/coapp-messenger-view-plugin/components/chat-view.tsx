import { Message, MessengerView } from "@1io/coapp-messenger-view";
import { LevelStack } from "@1io/kui-level-stack";
import { CID } from "multiformats";
import * as React from "react";
import { useDispatch, useSelector } from "react-redux";
import { identity } from "rxjs";
import { invokeResolveCid } from "../../../library/invoke-get.js";
import { MessengerViewActionType, MessengerViewLoadMoreEventsAction, MessengerViewSendAction } from "../actions/index.js";
import { resolveMatrixAction } from "../library/handle-matrix-event.js";
import { MessengerViewPluginState } from "../state/index.js";

export interface MessengerViewContainerProps {
  onBack: () => void;
}

function useIpld<T>(co: string, cids: CID[], deserialize: (v: any) => T | undefined): ReadonlyMap<CID, T | undefined> {
  const [ipldMap, setIpldMap] = React.useState<Map<CID, T | undefined>>(new Map());
  React.useEffect(() => {
    // cancel flag because component may unmount before fetch is done after which state changes become illegal
    let canceled = false;
    // async function that fetches the messages
    const fetchCids = async () => {
      const newMap = new Map();
      for (let cid of cids) {
        if (ipldMap.has(cid)) {
          // use value from old map
          newMap.set(cid, ipldMap.get(cid));
        } else {
          // fetch cid if not already loaded
          const ipld = await invokeResolveCid(co, cid);
          newMap.set(cid, deserialize(ipld));
        }
      }
      // update map if component is still mounted
      if (!canceled) {
        setIpldMap(newMap);
      }
    };
    // call async fetch function
    fetchCids();
    return () => {
      canceled = true;
    }
  }, [cids]);
  return ipldMap;
}

export function MessengerViewContainer(props: MessengerViewContainerProps) {

  const dispatch = useDispatch();
  const [message, setMessage] = React.useState("");
  const messageCids = useSelector((state: MessengerViewPluginState) => state.messages);
  const chatName = useSelector((state: MessengerViewPluginState) => state.chatName);
  const co = useSelector((state: MessengerViewPluginState) => state.co);

  const messageMap = useIpld<Message>(co, messageCids, resolveMatrixAction);
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
    dispatch<MessengerViewSendAction>({
      type: MessengerViewActionType.Send,
      payload: { message }
    });
    setMessage("");
  }

  const onScrollTop = () => {
    dispatch(identity<MessengerViewLoadMoreEventsAction>({
      payload: { count: 30 },
      type: MessengerViewActionType.LoadMoreEvents,
    }));
  }

  return <LevelStack style={{ width: "100%", height: "100%" }}>
    <MessengerView
      chatInput={message}
      chatName={chatName}
      onChatInput={setMessage}
      messages={messages}
      onSendMessage={onSendMessage}
      onBack={props.onBack}
      onScrollTop={onScrollTop}
    />
  </ LevelStack>;

}

