import { CoId } from "./co.js";

/** Enum (serde external-tag) */
export type Network =
  | { DidDiscovery: NetworkDidDiscovery }
  | { CoHeads: NetworkCoHeads }
  | { Rendezvous: NetworkRendezvous }
  | { Peer: NetworkPeer };

/** DID Discovery protocol. */
export interface NetworkDidDiscovery {
  /** The GossipSub topic used for DidDiscovery messages. Default: "co-contact". */
  topic?: string; // Option<String>
  /** The DID to be discovered. */
  did: string;
}

/** CO Heads protocol. */
export interface NetworkCoHeads {
  /** The GossipSub topic used for Heads messages. Default: "co-{co.id}". */
  topic?: string; // Option<String>
  /** The CO to be discovered. */
  id: CoId;
}

/** Rendezvous protocol. */
export interface NetworkRendezvous {
  /** The namespace to register to. */
  namespace: string;
  /** Rendezvous node multi-addresses. */
  addresses: string[];
}

/** Direct peer connection. */
export interface NetworkPeer {
  /** libp2p PeerId as bytes. (serde_json: number[] unless using serde_bytes for base64) */
  peer: number[]; // Vec<u8>
  /** Optional known multi-addresses. */
  addresses: string[];
}
