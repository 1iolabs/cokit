import { BlockStorage, CoMap } from "co-js";
import { useEffect, useState } from "react";

/**
 * IPLD can only use string keys for maps
 */
export function useCollectCoMap<V>(map: CoMap | undefined, storage: BlockStorage | undefined): Map<string, V> {
  const [tasks, setTasks] = useState<Map<string, V>>(new Map());
  useEffect(() => {
    async function fetchTasks() {
      if (map !== undefined && storage !== undefined) {
        const stream = map.stream(storage);
        const t = new Map();
        try {
          for await (const value of stream) {
            t.set(value[0], value[1]);
          }
          setTasks(t);
        } catch (err) {
          console.error(err, t);
        }
      }
    }
    fetchTasks();
  }, [map, storage]);
  return tasks;
}
