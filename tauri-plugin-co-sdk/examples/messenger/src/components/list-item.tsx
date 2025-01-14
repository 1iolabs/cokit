import { Badge, Color } from "@1io/kui-badge";
import React, { MouseEventHandler } from "react";
import { Chat } from "../state/index.js";

export interface ListItemProps {
    chat: Chat;
    openChat: MouseEventHandler<HTMLDivElement>;
}

export function ListItem(props: ListItemProps) {
    return <div
        onClick={props.openChat}
        className={"listButton"}>
        <div className={"chatHeader"}>
            <div children={props.chat.name} />
            {props.chat.newMessages > 0
                ? <Badge
                    className={"newMessages"}
                    badgeCount={props.chat.newMessages}
                    color={Color.Red}
                />
                : null}
        </div>
        {props.chat.lastMessage
            ? <div className={"lastMessage"} children={props.chat.lastMessage} />
            : null}
    </div>;
}