import { Message, MessengerView } from "@1io/coapp-messenger-view";
import { usePluginActionApi, WellKnownTags } from "@1io/kui-application-sdk";
import "@1io/packaging-utils/svg";
import * as React from "react";
import { useDispatch, useSelector } from "react-redux";
import DefaultProfilePic from "../../../assets/Users_24.svg";
import { buildCoCoreId } from "../../../library/core-id.js";
import { useCoCoreActions } from "../../../library/use-co-core-actions.js";
import { useCoIpld } from "../../../library/use-co-ipld.js";
import { useCoSession } from "../../../library/use-co-session.js";
import { useCo } from "../../../library/use-co.js";
import { useCoCore } from "../../../library/use-co-core.js";
import { COAppChatsListApi } from "../../coapp-chatslist-plugin/api/index.js";
import { coappChatsListPluginId } from "../../coapp-chatslist-plugin/types/plugin.js";
import { MessengerViewActionType, MessengerViewSendAction } from "../actions/index.js";
import { resolveMatrixAction } from "../library/handle-matrix-event.js";
import { MessengerViewPluginState } from "../types/state.js";
import { useResolvedCid } from "../../../library/use-resolved-cid.js";
import { Room } from "../../../../../../dist-js/index.js";

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

  const roomCoreState = useResolvedCid<Room.Room>(roomCoreCid, session);

  // how many actions should be loaded
  const [actionCount, setActionCount] = React.useState(0);

  // get actions from co log
  const actions = useCoCoreActions(coId, coreId, coHeads, session, actionCount)?.actions;

  // get and resolve messages
  const messageMap = useCoIpld<Message>(actions ?? [], resolveMatrixAction, session, ownIdentity);
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
