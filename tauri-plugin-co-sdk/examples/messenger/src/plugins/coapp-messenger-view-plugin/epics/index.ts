import { combineEpics } from "@1io/kui-application-sdk";
import { MessengerViewEpicType } from "../types/plugin";
import { initEpic } from "./init";
import { sendEpic } from "./send";
import { subscribeTauriEventEpic } from "./subscribe-tauri-events";

export const messengerViewEpic: MessengerViewEpicType = combineEpics<MessengerViewEpicType>(initEpic, sendEpic, subscribeTauriEventEpic);
