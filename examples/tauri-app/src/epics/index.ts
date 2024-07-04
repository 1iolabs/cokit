import { combineEpics } from "@1io/kui-application-sdk";
import { MessengerEpicType } from "../types/plugin";
import { initEpic } from "./init";
import { sendEpic } from "./send";
import { subscribeTauriEventEpic } from "./subscribe-tauri-events";

export const messengerEpic: MessengerEpicType = combineEpics<MessengerEpicType>(initEpic, sendEpic, subscribeTauriEventEpic);
