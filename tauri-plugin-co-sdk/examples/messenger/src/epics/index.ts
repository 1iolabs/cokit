import { combineEpics } from "@1io/kui-application-sdk";
import { ChatsListEpicType } from "../types/plugin.js";
import { groupDetailsEpic } from "./group-details.js";
import { initEpic } from "./init.js";
import { loadChatsEpic } from "./load-chats.js";
import { openChatEpic } from "./open-chat.js";
import { subscribeChatsEpic } from "./subscribe-chats.js";

export const chatsListEpic: ChatsListEpicType = combineEpics<ChatsListEpicType>(
    initEpic,
    loadChatsEpic,
    openChatEpic,
    subscribeChatsEpic,
    groupDetailsEpic,
);
