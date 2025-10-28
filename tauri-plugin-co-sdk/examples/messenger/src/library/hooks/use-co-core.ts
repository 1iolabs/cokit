import { CID } from "multiformats";
import React from "react";
import { resolveCid } from "../../../../../dist-js/index.js";

export function useCoCore(coCid: CID | undefined, coreId: string, session: string | undefined): CID | undefined {
  const [coreState, setCoreState] = React.useState<CID | undefined>(undefined);
  React.useEffect(() => {
    console.log("use co core");
    let canceled = false;
    async function getCoreCid() {
      if (session !== undefined && coCid !== undefined && !canceled) {
        const resolvedCoState = await resolveCid(session, coCid);
        const coreCid = resolvedCoState?.c?.[coreId]?.state;
        if (coreCid !== undefined && coreCid !== null) {
          setCoreState(coreCid);
        }
      }
    }
    getCoreCid();
    return () => {
      canceled = true;
    };
  }, [coCid, coreId, session]);
  return coreState;
}
