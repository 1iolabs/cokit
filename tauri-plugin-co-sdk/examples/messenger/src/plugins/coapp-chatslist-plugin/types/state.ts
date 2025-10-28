import { PluginId } from "@1io/kui-application-sdk";

export interface LoadedChatPlugin {
  readonly chatId: string;
  readonly pluginId: PluginId;
}

export interface ChatsListPluginState {
  readonly loadedChats: ReadonlyArray<LoadedChatPlugin>;
  readonly selectedChat?: string;
  readonly priorityPluginiId?: PluginId;
  readonly identity?: string;
  readonly dialog?: PluginId;
}

export interface ChatsListPluginPublicState extends Pick<ChatsListPluginState, "identity"> {}
