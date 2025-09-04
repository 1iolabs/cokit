import { CID } from "multiformats";
import React from "react";
import { resolveCid } from "../../../../dist-js/index.js";

export function useCoIpld<T>(
  cids: CID[],
  deserialize: (v: any, ownIdentity: string) => T | undefined,
  sessionId?: string,
  ownIdentity?: string,
): ReadonlyMap<CID, T | undefined> {
  const [ipldMap, setIpldMap] = React.useState<Map<CID, T | undefined>>(new Map());
  React.useEffect(() => {
    // cancel flag because component may unmount before fetch is done after which state changes become illegal
    let canceled = false;
    // async function that fetches the messages
    const fetchCids = async () => {
      if (sessionId === undefined || ownIdentity === undefined) {
        return;
      }
      const newMap = new Map();
      for (const cid of cids) {
        if (ipldMap.has(cid)) {
          // use value from old map
          newMap.set(cid, ipldMap.get(cid));
        } else {
          // fetch cid if not already loaded
          const ipld = await resolveCid(sessionId, cid);
          newMap.set(cid, deserialize(ipld, ownIdentity));
        }
      }
      // update map if component is still mounted
      if (!canceled) {
        setIpldMap(newMap);
      }
    };
    // call async fetch function
    fetchCids();
    // return deconstructor to cancel ongoing operations
    return () => {
      canceled = true;
    };
  }, [cids.length, sessionId]);
  return ipldMap;
}
