import { CID } from "multiformats";
import React from "react";
import { resolveCid } from "../../invoke-utils.js";

export function useResolveCid<T = any>(cid: CID | undefined | null, session: string | undefined): T | undefined {
  const [state, setState] = React.useState<T | undefined>(undefined);
  React.useEffect(() => {
    async function resolveCoCid() {
      if (cid !== undefined && cid !== null && session !== undefined) {
        setState(await resolveCid(session, cid));
      }
    }
    resolveCoCid();
  }, [cid, session]);
  return state;
}
