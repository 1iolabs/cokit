import { Message, MessengerView } from "@1io/coapp-messenger-view";
import { usePluginActionApi, WellKnownTags } from "@1io/kui-application-sdk";
import "@1io/packaging-utils/svg";
import * as React from "react";
import { useDispatch, useSelector } from "react-redux";
import { Room } from "../../../../../../dist-js/index.js";
import DefaultProfilePic from "../../../assets/Users_24.svg";
import { buildCoCoreId } from "../../../library/core-id.js";
import { isMessage, ReadReceipt, resolveMatrixAction } from "../../../library/handle-matrix-event";
import { useCoCoreActions } from "../../../library/hooks/use-co-core-actions.js";
import { useCoCore } from "../../../library/hooks/use-co-core.js";
import { useCoIpld } from "../../../library/hooks/use-co-ipld.js";
import { useCoSession } from "../../../library/hooks/use-co-session.js";
import { useCo } from "../../../library/hooks/use-co.js";
import { useResolveCid } from "../../../library/hooks/use-resolve-cid.js";
import { invokePushMessage } from "../../../library/invoke-push.js";
import { COAppChatsListApi } from "../../coapp-chatslist-plugin/api/index.js";
import { coappChatsListPluginId } from "../../coapp-chatslist-plugin/types/plugin.js";
import { MessengerViewPluginState } from "../types/state.js";

// the number of additional actions that should be loaded when scrolling to the top of the page
const MESSAGE_PAGING_COUNT = 30;

export interface MessengerViewContainerProps {
  onBack: () => void;
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
  const [coStateCid, coHeads] = useCo(coId);

  const session = useCoSession(coId);

  // get room core state cid
  const roomCoreCid = useCoCore(coStateCid, coreId, session);

  const roomCoreState = useResolveCid<Room.Room>(roomCoreCid, session);

  // how many actions should be loaded
  const [actionCount, setActionCount] = React.useState(0);

  // get actions from co log
  const actions = useCoCoreActions(coId, coreId, coHeads, session, actionCount)?.actions;

  // get and resolve messages
  const messageMap = useCoIpld(actions ?? [], resolveMatrixAction, session, ownIdentity);
  const [messages, _readReceipt] = React.useMemo(() => {
    const m: Message[] = [];
    let r: ReadReceipt | undefined = undefined;
    for (const v of messageMap.values()) {
      if (v !== undefined) {
        if (isMessage(v)) {
          m.push(v);
        } else if (r === undefined) {
          // save first read receipt
          r = v;
        }
      }
    }
    return [m.reverse(), r];
  }, [messageMap]);

  // React.useEffect(() => {
  //   // Chat window is open -> always keep read receipt on newest message
  //   // TODO check for focus and scroll pos
  //   if (ownIdentity !== undefined) {
  //     if (messages.length > 0 && messages[messages.length - 1]!.key !== readReceipt?.messageId) {
  //       invokePushReadReceipt(coId, coreId, ownIdentity, messages[messages.length - 1]!.key, session);
  //     }
  //   }
  // }, [messages, readReceipt]);

  // currently typed message
  const [message, setMessage] = React.useState("");

  // send message handler
  const onSendMessage = React.useCallback(async () => {
    // don't send empty messages
    if (message !== "" && ownIdentity !== undefined && session !== undefined) {
      // TODO either reduce ait time to receive messages or fix temp messages
      const tmpMessage = invokePushMessage(message, coId, coreId, ownIdentity, session);
      messages.push(tmpMessage);
      setActionCount((c) => c + 1);
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
