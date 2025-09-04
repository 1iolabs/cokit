import { CID } from "multiformats";
import React from "react";
import { resolveCid } from "../../../../dist-js/index.js";

export function useResolvedCid<T = any>(cid: CID | undefined, session: string | undefined): T | undefined {
  const [state, setState] = React.useState<T | undefined>(undefined);
  React.useEffect(() => {
    async function resolveCoCid() {
      if (cid !== undefined && session !== undefined) {
        setState(await resolveCid(session, cid));
      }
    }
    resolveCoCid();
  }, [cid, session]);
  return state;
}
