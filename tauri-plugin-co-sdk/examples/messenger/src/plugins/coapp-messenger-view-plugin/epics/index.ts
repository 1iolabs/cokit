import { combineEpics } from "@1io/kui-application-sdk";
import { MessengerViewEpicType } from "../types/plugin.js";
import { initEpic } from "./init.js";
import { loadMoreEventsEpic } from "./load-more-events.js";
import { sendEpic } from "./send.js";
import { subscribeTauriEventEpic } from "./subscribe-tauri-events.js";

export const messengerViewEpic: MessengerViewEpicType = combineEpics<MessengerViewEpicType>(
    initEpic,
    subscribeTauriEventEpic,
    sendEpic,
    loadMoreEventsEpic,
);
