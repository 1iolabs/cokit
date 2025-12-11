import React from "react";
import { fromEventPattern } from "rxjs";
import { sessionOpen, sessionClose } from "../../invoke-utils.js";

export function useCoSession(co: string): string | undefined {
  const [session, setSession] = React.useState<string | undefined>();
  const [sessionError, setSessionError] = React.useState<Error | undefined>(undefined);
  React.useEffect(() => {
    const sessionSubscription = fromEventPattern<string>(
      async (handler) => {
        const s = await sessionOpen(co);
        handler(s);
        return s;
      },
      async (_, sessionPromise) => {
        const s = await sessionPromise;
        await sessionClose(s);
      },
    ).subscribe({
      next: (sessionEvent) => {
        setSession(sessionEvent);
      },
      error(err) {
        setSessionError(err);
      },
    });

    return () => {
      sessionSubscription.unsubscribe();
    };
  }, [co]);
  if (sessionError !== undefined) {
    throw sessionError;
  }
  return session;
}
