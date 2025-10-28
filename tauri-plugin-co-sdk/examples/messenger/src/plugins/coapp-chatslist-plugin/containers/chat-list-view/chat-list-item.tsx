import { Chat, ChatListItem, ChatListItemProps } from "@1io/coapp-chatlist-view";
import { extractKuiStyleRenderFeatures, extractSelectionStateFeatures } from "@1io/kui-common";
import { extractClickFeatures, extractIconsFeatures } from "@1io/kui-list";
import React from "react";
import { Room } from "../../../../../../../dist-js/index.js";
import GroupDefaultPic from "../../../../assets/Users_48.svg";
import { splitCoCoreId } from "../../../../library/core-id.js";
import { useCo } from "../../../../library/hooks/use-co.js";
import { useCoCore } from "../../../../library/hooks/use-co-core.js";
import { useCoSession } from "../../../../library/hooks/use-co-session.js";
import { useResolveCid } from "../../../../library/hooks/use-resolve-cid.js";
import { useCoCoreActions } from "../../../../library/hooks/use-co-core-actions.js";
import { useCoIpld } from "../../../../library/hooks/use-co-ipld.js";
import { isMessage, ReadReceipt, resolveMatrixAction } from "../../../../library/handle-matrix-event.js";
import { useSelector } from "react-redux";
import { ChatsListPluginState } from "../../types/state.js";
import { Message } from "@1io/coapp-messenger-view";

export function ChatListItemIdWrapper(props: ChatListItemProps<string>) {
  const coCoreId = splitCoCoreId(props.item);
  if (coCoreId === undefined) {
    throw new Error("Invalid co core ids");
  }
  const identity = useSelector((state: ChatsListPluginState) => state.identity);
  const coSession = useCoSession(coCoreId.coId);
  const [coStateCid, heads] = useCo(coCoreId.coId);
  const roomCoreCid = useCoCore(coStateCid, coCoreId.coreId, coSession);
  const roomCore = useResolveCid<Room.Room>(roomCoreCid, coSession);

  const actions = useCoCoreActions(coCoreId.coId, coCoreId.coreId, heads, coSession, 99)?.actions;
  const messageMap = useCoIpld(actions, resolveMatrixAction, coSession, identity);
  const lastMessage = React.useMemo(() => {
    for (const v of messageMap.values()) {
      if (v !== undefined && isMessage(v)) {
        return v;
      }
    }
    return undefined;
  }, [messageMap]);

  // get and resolve messages
  const unreadMessageCount = React.useMemo(() => {
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
    console.log(m, r);
    if (r === undefined) {
      return m.length;
    }
    const index = m.findIndex((i) => i.key === r.messageId);
    return index === -1 ? 99 : index;
  }, [messageMap.size]);

  const chat: Chat = {
    avatar: GroupDefaultPic, // TODO correct pic
    id: props.item,
    name: roomCore?.name ?? "Unnamed group",
    newMessages: unreadMessageCount,
    lastMessage,
  };

  return (
    <ChatListItem
      {...extractKuiStyleRenderFeatures(props)}
      {...extractClickFeatures(props)}
      {...extractSelectionStateFeatures(props)}
      {...extractIconsFeatures(props)}
      key={props.item}
      item={chat}
    />
  );
}
