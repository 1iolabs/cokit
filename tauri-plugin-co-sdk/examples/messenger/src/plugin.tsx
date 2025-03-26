import { COApp1ioClassName } from "@1io/coapp-style";
import { TagList } from "@1io/kui-application-sdk";
import { clsx } from "clsx";
import React from "react";
import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import { ChatListViewContainer } from "./containers/chat-list-view.js";
import { CreateIdentity } from "./containers/create-identity.js";
import { chatsListEpic } from "./epics/index.js";
import { chatsListReducer } from "./reducers/index.js";
import { ChatsListPlugin } from "./types/plugin.js";

document.body.className = clsx(document.body.className, COApp1ioClassName);

export default function plugin(pluginTags: TagList): ChatsListPlugin {
    return {
        context: (context) => context,
        epic: chatsListEpic,
        reducer: chatsListReducer,
        render: (renderTags) => {
            return (<BrowserRouter>
                <Routes>
                    <Route path="/" element={<Navigate to="chats" />} />
                    <Route path="chats" element={<ChatListViewContainer />} />
                    <Route path="identity" element={<CreateIdentity />} />
                    <Route path="*" element={"404"} />
                </Routes>
            </BrowserRouter>);
        },
        tags: [{ key: "type", value: "coapp-chats-list" }, ...pluginTags],

    };
}
