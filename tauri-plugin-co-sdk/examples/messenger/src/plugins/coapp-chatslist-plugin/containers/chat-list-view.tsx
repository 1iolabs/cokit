import { buildChatListItemRenderProps, ChatListView, NoChatSelected } from "@1io/coapp-chatlist-view";
import { PluginView } from "@1io/kui-application-sdk";
import { LevelPortal } from "@1io/kui-level-stack";
import React from "react";
import { useDispatch, useSelector } from "react-redux";
import { identity } from "rxjs";
import { useCoCore } from "../../../library/use-co-core.js";
import { useCoSession } from "../../../library/use-co-session.js";
import { useCo } from "../../../library/use-co.js";
import { useCoIds } from "../../../library/use-cos.js";
import { useFilteredCores } from "../../../library/use-filtered-cores.js";
import { useResolvedCid } from "../../../library/use-resolved-cid.js";
import {
  ChatsListActionType,
  ChatsListCopyIdentityAction,
  ChatsListOpenChatAction,
  ChatsListOpenChatDetailsAction,
  ChatsListSetDialogAction,
  ChatsListSetPriorityPlugin,
} from "../actions/index.js";
import { ChatsListPluginState } from "../types/state.js";
import { ChatListItemIdWrapper } from "./chat-list-view/chat-list-item.js";

const ROOM_CORE_TAGS = [["core", "co-core-room"]];

export interface ChatListViewContainerProps {}

export function ChatListViewContainer(props: ChatListViewContainerProps) {
  const dispatch = useDispatch();

  const localCoSession = useCoSession("local");
  const [localCoState] = useCo("local");
  const membershipsStateCid = useCoCore(localCoState, "membership", localCoSession);
  const membershipsState = useResolvedCid(membershipsStateCid, localCoSession);
  const coIds = useCoIds(membershipsState);
  let roomCoCoreIds = useFilteredCores(ROOM_CORE_TAGS, coIds);

  const selectedChat = useSelector((state: ChatsListPluginState) => state.selectedChat);
  const loadedChats = useSelector((state: ChatsListPluginState) => state.loadedChats);
  const priorityPlugin = useSelector((state: ChatsListPluginState) => state.priorityPluginiId);
  const pluginId = priorityPlugin ?? (selectedChat !== undefined ? loadedChats.get(selectedChat) : undefined);
  const dialogPluginId = useSelector((state: ChatsListPluginState) => state.dialog);

  console.log(loadedChats);

  const onOpenChat = React.useCallback((chat: string | undefined) => {
    if (chat !== undefined) {
      dispatch(identity<ChatsListOpenChatAction>({ payload: { chat }, type: ChatsListActionType.OpenChat }));
    }
  }, []);
  const onOpenChatDetails = () =>
    dispatch<ChatsListOpenChatDetailsAction>({
      payload: { coCoreId: undefined },
      type: ChatsListActionType.OpenChatDetails,
    });
  const onClosePlugin = () => {
    if (priorityPlugin !== undefined) {
      dispatch<ChatsListSetPriorityPlugin>({
        payload: { pluginId: undefined },
        type: ChatsListActionType.SetPriorityPlugin,
      });
    }
  };
  const onCloseDialog = () => {
    if (dialogPluginId !== undefined) {
      dispatch<ChatsListSetDialogAction>({
        payload: { dialogPluginId: undefined },
        type: ChatsListActionType.SetDialog,
      });
    }
  };
  const onCopyIdentity = () => {
    dispatch<ChatsListCopyIdentityAction>({ type: ChatsListActionType.CopyIdentity });
  };
  React.useEffect(() => {
    console.log("new room ids", roomCoCoreIds);
  }, [roomCoCoreIds]);
  return (
    <>
      <ChatListView<string>
        key={"list"}
        items={roomCoCoreIds}
        selectedChat={selectedChat}
        viewComponent={
          pluginId !== undefined ? (
            <PluginView props={{ onClose: onClosePlugin }} plugin={pluginId} />
          ) : (
            <NoChatSelected />
          )
        }
        onClickCopyId={onCopyIdentity}
        onClickCreateGroup={onOpenChatDetails}
        onSelectChat={onOpenChat}
        itemKey={(i: string) => i}
        itemLabel={(i: string) => i}
        renderItemComponent={(props) => {
          console.log("props", props);
          return <ChatListItemIdWrapper {...props!} key={props?.item} />;
        }}
        renderItemProps={() => buildChatListItemRenderProps}
        renderItemPropsExtra={[]}
      />
      <LevelPortal modal={dialogPluginId !== undefined}>
        {<PluginView props={{ onClose: onCloseDialog }} plugin={dialogPluginId} />}
      </LevelPortal>
    </>
  );
}
