import { TagList } from "@1io/kui-application-sdk";
import React from "react";
import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import { ListView } from "./components/chats-list.js";
import { chatsListEpic } from "./epics/index.js";
import { chatsListReducer } from "./reducers/index.js";
import { ChatsListPlugin } from "./types/plugin.js";


export default function plugin(pluginTags: TagList): ChatsListPlugin {
    return {
        context: (context) => context,
        epic: chatsListEpic,
        reducer: chatsListReducer,
        render: (renderTags) => {
            return (<BrowserRouter>
                <Routes>
                    <Route path="/" element={<Navigate to="chats" />} />
                    <Route path="chats" element={<ListView />} />
                </Routes>
            </BrowserRouter>);
        },
        tags: [{ key: "type", value: "coapp-chats-list" }, ...pluginTags],

    };
}
