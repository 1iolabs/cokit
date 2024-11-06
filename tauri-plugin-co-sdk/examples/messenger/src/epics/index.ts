import { combineEpics } from "@1io/kui-application-sdk";
import { ChatsListEpicType } from "../types/plugin";
import { initEpic } from "./init";
import { openChatEpic } from "./open-chat";
import { subscribeChatsEpic } from "./subscribe-chats";

export const chatsListEpic: ChatsListEpicType = combineEpics<ChatsListEpicType>(
    initEpic,
    openChatEpic,
    subscribeChatsEpic,
);
