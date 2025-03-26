import { Chat, ChatListView, NoChatSelected } from "@1io/coapp-chatlist-view";
import { PluginView } from "@1io/kui-application-sdk";
import React from "react";
import { useDispatch, useSelector } from "react-redux";
import { identity } from "rxjs";
import { ChatsListActionType, ChatsListOpenChatAction } from "../actions/index.js";
import { ChatsListPluginState } from "../state/index.js";

export interface ChatListViewContainerProps { }

export function ChatListViewContainer(props: ChatListViewContainerProps) {
    const dispatch = useDispatch();
    const chats = useSelector((state: ChatsListPluginState) => state.chats);
    const pluginId = useSelector((state: ChatsListPluginState) => state.activePlugin);
    const onOpenChat = (chat: Chat | undefined) => {
        if (chat) {
            dispatch(identity<ChatsListOpenChatAction>({ payload: { chat }, type: ChatsListActionType.OpenChat }));
        }
    };
    return <ChatListView
        chats={chats}
        selectedChat={undefined}
        viewComponent={
            pluginId
                ? <PluginView plugin={pluginId} />
                : <NoChatSelected />
        }
        onClickCopyId={() => undefined} // TODO
        onClickCreateGroup={() => undefined} // TODO
        onSelectChat={onOpenChat}
    />;
}
