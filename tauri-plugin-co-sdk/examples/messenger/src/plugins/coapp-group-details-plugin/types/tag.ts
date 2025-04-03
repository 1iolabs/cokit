import { Tag, WellKnownTags } from "@1io/kui-application-sdk";

export const coappGroupViewPluginId = "coapp-group-view";

export type GroupViewPluginTagType = Tag<WellKnownTags.Type, typeof coappGroupViewPluginId>;
export const groupViewPluginTag: GroupViewPluginTagType = { key: WellKnownTags.Type, value: coappGroupViewPluginId };

export type GroupViewPluginRoomCoreIdTag = Tag<"roomCoreId", string>;
