import { PluginView } from "@1io/kui-application-sdk";
import React from "react";
import { useDispatch, useSelector } from "react-redux";
import { identity } from "rxjs";
import { ChatsListActionType, ChatsListActivatePluginAction, ChatsListOpenChatAction } from "../actions/index.js";
import { Chat, ChatsListPluginState } from "../state/index.js";
import "./chats-list.css";
import { ListItem } from "./list-item.js";

export interface ListViewProps { }

export function ListView(props: ListViewProps) {
    const dispatch = useDispatch();
    const chats = useSelector((state: ChatsListPluginState) => state.chats);
    const pluginId = useSelector((state: ChatsListPluginState) => state.activePlugin);
    const onOpenChat = (chat: Chat) => {
        dispatch(identity<ChatsListOpenChatAction>({ payload: { chat }, type: ChatsListActionType.OpenChat }));
    };
    const onBack = () => dispatch(identity<ChatsListActivatePluginAction>({
        payload: { pluginId: undefined },
        type: ChatsListActionType.ActivatePlugin
    }));
    return pluginId
        ? <PluginView plugin={pluginId} props={{ onBack }} />
        : <div className="chatsList">
            {chats.map((chat) => <ListItem key={chat.roomCoreId} chat={chat} openChat={() => onOpenChat(chat)} />)}
        </div>
}
