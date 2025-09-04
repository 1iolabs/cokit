import React from "react";
import { getResolvedCoState } from "./invoke-get.js";
import { buildCoCoreId } from "./core-id.js";
import { compareArrayItemsEqual } from "@1io/compare";

// returns true if and only if all of the tags in the pattern can be found in the given tags
function matchesPattern(tags: string[][], pattern: string[][]): boolean {
  return pattern.every((patternTag) => {
    const foundTag = tags.find((tag) => {
      return tag[0] === patternTag[0] && tag[1] === patternTag[1];
    });
    return foundTag !== undefined;
  });
}

export function useFilteredCores(tagsPattern: string[][], coIds: string[]): string[] {
  const [coCoreIds, setCoCoreIds] = React.useState<string[]>([]);
  React.useEffect(() => {
    async function loadCores() {
      const foundCoCoreIds: string[] = [];
      for (const coId of coIds) {
        const state = await getResolvedCoState(coId);
        if (state === undefined || state === null) {
          continue;
        }
        for (const [key, value] of Object.entries(state.c)) {
          // TODO remove any cast when js interfaces are done
          const v = value as any;
          if (v?.tags !== undefined && v?.tags !== null) {
            if (matchesPattern(v.tags, tagsPattern)) {
              foundCoCoreIds.push(buildCoCoreId(coId, key));
            }
          }
        }
      }
      if (!compareArrayItemsEqual(coCoreIds, foundCoCoreIds)) {
        setCoCoreIds(foundCoCoreIds);
      }
    }
    loadCores();
  }, [tagsPattern, coIds.length]);
  return coCoreIds;
}
