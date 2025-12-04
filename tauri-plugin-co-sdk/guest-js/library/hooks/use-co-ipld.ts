import { CID } from "multiformats";
import React from "react";
import { resolveCid } from "../../invoke-utils.js";

export function useCoIpld<T, E = []>(
  cids: CID[] | undefined,
  deserialize: (v: any, extras?: E) => T | undefined,
  sessionId?: string,
  extras?: E,
): ReadonlyMap<CID, T | undefined> {
  const [ipldMap, setIpldMap] = React.useState<Map<CID, T | undefined>>(new Map());
  React.useEffect(() => {
    // cancel flag because component may unmount before fetch is done after which state changes become illegal
    let canceled = false;
    // async function that fetches the messages
    const fetchCids = async () => {
      if (sessionId === undefined || cids === undefined) {
        return;
      }
      const newMap = new Map<CID, T>();
      for (const cid of cids) {
        if (ipldMap.has(cid)) {
          // use value from old map
          newMap.set(cid, ipldMap.get(cid)!);
        } else {
          // fetch cid if not already loaded
          const ipld = await resolveCid(sessionId, cid);
          const data = deserialize(ipld, extras);
          if (data !== undefined) {
            newMap.set(cid, data);
          }
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
  }, [cids, sessionId, extras]);
  return ipldMap;
}
