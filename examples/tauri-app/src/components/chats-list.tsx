import { PluginView } from "@1io/kui-application-sdk";
import { Button } from "@1io/kui-button";
import React from "react";
import { useDispatch, useSelector } from "react-redux";
import { identity } from "rxjs";
import { ChatsListActionType, ChatsListOpenChatAction } from "../actions";
import { ChatsListPluginState } from "../state";

export interface ListViewProps { }

export function ListView(props: ListViewProps) {
    const dispatch = useDispatch();
    const chats = useSelector((state: ChatsListPluginState) => state.chats);
    const pluginId = useSelector((state: ChatsListPluginState) => state.activePlugin);
    const onClickbutton = () => {
        dispatch(identity<ChatsListOpenChatAction>({ payload: { chat: "" }, type: ChatsListActionType.OpenChat }));
        console.log("ayo button", chats);
    };
    return pluginId
        ? <PluginView plugin={pluginId} />
        : <Button label="show room" onClick={onClickbutton}></Button>;
}
