import { CID } from "multiformats";
import { resolveCid } from "../invoke-utils";

export interface Node<I> {
  n: CID[] | undefined;
  l: I[] | undefined;
}

/// temp class that will be replaced by web assembly calls to rust primitves
export class DagList<I> {
  private nodes: CID[];
  private elements: I[];
  private session: string;

  public constructor(root: Node<I>, session: string) {
    // copy root node items
    this.nodes = root.n ? [...root.n] : [];
    this.elements = root.l ? [...root.l] : [];
    this.session = session;
  }

  /// tries to load more elements
  /// returns true if elements were loaded, false otherwise
  async resolveNext(): Promise<boolean> {
    const nodeCid = this.nodes.pop();
    if (nodeCid === undefined) {
      return false;
    }
    const newNode: Node<I> = await resolveCid(this.session, nodeCid);
    if (newNode?.n !== undefined) {
      this.nodes.push(...newNode.n);
    }
    if (newNode?.l !== undefined) {
      this.elements.push(...newNode.l);
    }
    return true;
  }

  async get(index: number): Promise<I | undefined> {
    do {
      if (this.elements.length > index) {
        return this.elements[index];
      }
    } while ((await this.resolveNext()) === true);
    return undefined;
  }

  async find(predicate: (i: I) => boolean): Promise<I | undefined> {
    let count = 0;
    while (true) {
      const next = await this.get(count);
      // no more items
      if (next === undefined) {
        return undefined;
      }
      if (predicate(next)) {
        return next;
      }
      count++;
    }
  }
}
