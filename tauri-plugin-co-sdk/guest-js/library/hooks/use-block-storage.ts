import { BlockStorage } from "@1io/co-js";
import { CID } from "multiformats";
import { useMemo } from "react";
import { storageGet, storageSet } from "../../invoke-utils.js";

export function useBlockStorage(session?: string) {
  return useMemo(() => {
    if (session !== undefined) {
      return new BlockStorage(
        async (cidBinary: Uint8Array) => {
          const cid = CID.decode(cidBinary);
          const block = await storageGet(session, cid);
          return block;
        },
        async (cid: Uint8Array, data: Uint8Array): Promise<Uint8Array> => {
          const cidCheck = await storageSet(session, CID.decode(cid), data);
          return cidCheck.bytes;
        },
      );
    }
    return undefined;
  }, [session]);
}
