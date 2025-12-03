import { createIdentity, getCoreState, Keystore, resolveCid } from "@1io/tauri-plugin-co-sdk-api";
import React, { useEffect } from "react";
import { DagList } from "../dag-list";
import { Did } from "../../types";
import { useCoSession } from "./use-co-session";

export function useDidKeyIdentity(name: string): Did | undefined {
  const [identity, setIdentity] = React.useState<Did | undefined>();
  const sessionId = useCoSession("local");
  useEffect(() => {
    async function getIdentity() {
      if (sessionId === undefined) {
        return;
      }
      const keystoreState = await getCoreState("local", "keystore", sessionId);
      if (keystoreState?.keys === undefined || keystoreState === null) {
        return;
      }
      const keyStoreKeys = await resolveCid(sessionId, keystoreState.keys);
      // TODO use wasm CoList instead
      // rust map is lsm tree so this might not always work
      const dagList = new DagList<[string, { v?: Keystore.Key; t?: undefined }]>(keyStoreKeys.a, sessionId);
      const messengerIdentity = await dagList.find((i) => i[1].v?.name === name);
      if (messengerIdentity === undefined) {
        const newIdentity = await createIdentity(name);
        setIdentity(newIdentity);
      } else {
        setIdentity(messengerIdentity?.[0]);
      }
    }
    getIdentity();
  }, [name, sessionId]);
  return identity;
}
