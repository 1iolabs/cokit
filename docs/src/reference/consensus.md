# Consensus

Consensus is the validated state of a CO.

## What makes a Consensus in CO-kit
Consensus in CO-kit provides finality.
When heads from different participants are joined, the event order in the [Log](./log.md#example) may change.
This is necessary to make the Merkle-CRDT work technically.

The consensus, on the other hand, is used to allow the network of participants of an CO to commit to an known state/heads combination.
After a successful consensus round (depending on the consensus algorithm), there are no more heads accepted that would alter the sorting of events before this point.

## Checkpoint
For each set of heads, a materialized state is calculated.
A checkpoint (or snapshot) is the combination of this state and heads.
Each successful consensus round produces a checkpoint.
For example:
- a checkpoint functions like a block in a Blockchain, but it's more versatile because it is not dependent on a predefined mining period - but rather works on demand.

## Consensus on demand (CoD)
A consensus round can be triggered manually on demand.
One participant can request finality at any given time.
Other participants that received the initial request, then collaborate and sign the checkpoint.

## Finality
Finality means a certain checkpoint is agreed upon and no events can be inserted by the conflict resolution algorithm before it.

## Trust
Finality is cryptographically verifiable once reached and therefore can be trusted to be immutable among CO participants.

## Optional
Consensus algorithms in a CO are completely optional.
The [Log](./log.md) with the [Merkle-CRDT](../glossary/glossary.md#merkle-crdt) solves the technical part of syncing, conflict handling and validation.
If no other mechanism is applied (proof of authority, etc.), the Merkle-CRDT serves as a single source of truth.
Depending on your project requirements, you can implement any other consensus mechanism.

## Asynchronous
In CO-kit consensus is asynchronous and on-demand, meaning users only have to wait for it, if it is really needed.
While conventional databases normally provide finality by default (when a change is written it's final) this comes at the cost that every change losses time by having a lot of overhead (latency, routing, compute, ...).

## Proof of authority
Proof of authority is the built-in consensus mechanism used in CO-kit.

It is implemented in the `co-core-poa` Core.

When the creator of a CO adds this Core the authority can be specified as a list of DIDs.
This authority is then responsible for voting and once a majority is reached on a checkpoint, it is finalized.
The POA allows for Byzantine Fault Tolerance (BFT) when configured with a majority of at least two-thirds of the authority.

By default `2/3` majority is used.

### Use Cases
With this mechanism new and classical use cases can be supported.

#### Client / Server
Just define the Server (or Servers) as the authority then clients can be sure that the finalized state is known by the server.

#### Receipts
When two parties buy/sell something from each other, a CO can be used as a receipt between them. After all data has been captured, both can sign and therefore finalize the CO and receive a verifiable proof that a transaction took place.

#### Multi Region Cloud Service
When collaborating globally, even across continents (e.g. Europe, North America, Asia), the majority can be formed by two of the continents and thus making sure a change has been globally replicated/accepted upon.

## See also
- [co-core-poa](../reference/core.md#co-core-poa)
- [guards](../reference/guards.md)
	