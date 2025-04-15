import { Chat, ChatListView, NoChatSelected } from "@1io/coapp-chatlist-view";
import { PluginView } from "@1io/kui-application-sdk";
import React from "react";
import { useDispatch, useSelector } from "react-redux";
import { identity } from "rxjs";
import { ChatsListActionType, ChatsListOpenChatAction, ChatsListOpenChatDetailsAction, ChatsListSetPriorityPlugin } from "../actions/index.js";
import { ChatsListPluginState } from "../state/index.js";

export interface ChatListViewContainerProps { }

export function ChatListViewContainer(props: ChatListViewContainerProps) {
    const dispatch = useDispatch();
    const chats = useSelector((state: ChatsListPluginState) => state.chats);
    const selectedChatId = useSelector((state: ChatsListPluginState) => state.selectedChat);
    const selectedChat = selectedChatId ? chats.find((c) => c.id === selectedChatId) : undefined;
    const loadedChats = useSelector((state: ChatsListPluginState) => state.loadedChats);
    const priorityPlugin = useSelector((state: ChatsListPluginState) => state.priorityPluginiId);
    const pluginId = priorityPlugin ?? (selectedChatId ? loadedChats.get(selectedChatId) : undefined);
    const onOpenChat = (chat: Chat | undefined) => {
        if (chat) {
            dispatch(identity<ChatsListOpenChatAction>({ payload: { chat }, type: ChatsListActionType.OpenChat }));
        }
    };
    const onOpenChatDetails = () => dispatch<ChatsListOpenChatDetailsAction>({
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
    return <ChatListView
        chats={chats}
        selectedChat={selectedChat}
        viewComponent={
            pluginId
                ? <PluginView props={{ onClose: onClosePlugin }} plugin={pluginId} />
                : <NoChatSelected />
        }
        onClickCopyId={() => undefined} // TODO
        onClickCreateGroup={onOpenChatDetails}
        onSelectChat={onOpenChat}
    />;
}
