import React from "react";
import { compareArrayItemsEqual } from "@1io/compare";
import { CoCore, getResolvedCoState } from "../../invoke-get-utils";

// returns true if and only if all of the tags in the pattern can be found in the given tags
function matchesPattern(tags: string[][], pattern: string[][]): boolean {
  return pattern.every((patternTag) => {
    const foundTag = tags.find((tag) => {
      return tag[0] === patternTag[0] && tag[1] === patternTag[1];
    });
    return foundTag !== undefined;
  });
}

export function useFilteredCores(tagsPattern: string[][], coIds: string[]): CoCore[] {
  const [coCoreIds, setCoCoreIds] = React.useState<CoCore[]>([]);
  React.useEffect(() => {
    async function loadCores() {
      const foundCoCoreIds: CoCore[] = [];
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
              foundCoCoreIds.push({ coId, coreId: key });
            }
          }
        }
      }
      if (!compareArrayItemsEqual(coCoreIds, foundCoCoreIds)) {
        setCoCoreIds(foundCoCoreIds);
      }
    }
    loadCores();
  }, [coIds]);
  return coCoreIds;
}
