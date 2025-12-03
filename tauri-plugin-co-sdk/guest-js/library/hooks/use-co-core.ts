import { CID } from "multiformats";
import React from "react";
import { resolveCid } from "../../invoke-utils";

export function useCoCore(coCid: CID | undefined, coreId: string, session: string | undefined): CID | undefined | null {
  const [coreState, setCoreState] = React.useState<CID | undefined | null>(undefined);
  React.useEffect(() => {
    let canceled = false;
    async function getCoreCid() {
      if (session !== undefined && coCid !== undefined && !canceled) {
        const resolvedCoState = await resolveCid(session, coCid);
        const coreCid = resolvedCoState?.c?.[coreId]?.state;
        if (coreCid !== undefined) {
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
