import { Chat, ChatListItem, ChatListItemProps } from "@1io/coapp-chatlist-view";
import { extractKuiStyleRenderFeatures, extractSelectionStateFeatures } from "@1io/kui-common";
import { extractClickFeatures, extractIconsFeatures } from "@1io/kui-list";
import React from "react";
import { Room } from "../../../../../../../dist-js/index.js";
import GroupDefaultPic from "../../../../assets/Users_48.svg";
import { splitCoCoreId } from "../../../../library/core-id.js";
import { useCo } from "../../../../library/use-co.js";
import { useCoCore } from "../../../../library/use-co-core.js";
import { useCoSession } from "../../../../library/use-co-session.js";
import { useResolvedCid } from "../../../../library/use-resolved-cid.js";

export function ChatListItemIdWrapper(props: ChatListItemProps<string>) {
  const coCoreId = splitCoCoreId(props.item);
  if (coCoreId === undefined) {
    throw new Error("Invalid co core ids");
  }
  const coSession = useCoSession(coCoreId.coId);
  const [co] = useCo(coCoreId.coId);
  const roomCoreCid = useCoCore(co, coCoreId.coreId, coSession);
  const roomCore = useResolvedCid<Room.Room>(roomCoreCid, coSession);
  const chat: Chat = {
    avatar: GroupDefaultPic, // TODO correct pic
    id: props.item,
    name: roomCore?.name ?? "Unnamed group",
    newMessages: 0, // TODO
  };
  React.useEffect(() => {
    console.log("mount", props);
    return () => console.log("unmount chatlist item");
  }, [props.item]);
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
