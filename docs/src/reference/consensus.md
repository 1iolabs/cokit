# Consensus

The Consensus is the mechanism to reach a validated CO state.

## What makes a Consensus in CO-kit
Consensus in CO-kit provides finality.  

When heads from different participants are joined, the event order in the [Log](../reference/log.md#example) may change.  
This is necessary, from a technical perspective, to make the [Merkle-CRDT](../glossary/glossary.md#merkle-crdt) work.

The Consensus, on the other hand, is used to allow the network of participants of a CO to commit to a known state/heads combination.  
After a successful consensus round (depending on the consensus algorithm), there are no more heads accepted that would alter the sorting of events before this point.  
At this point, a finalized state has been reached.

## Checkpoint
For each set of heads, a materialized state is calculated.  
A checkpoint (or snapshot) is the combination of this state and heads.  
Each successful consensus round produces a checkpoint.

For example:
- a checkpoint functions like a block in a Blockchain, but is more versatile because it's not dependent on a predefined mining period - but rather works on demand.

## Consensus on Demand (CoD)
A consensus round can be triggered manually on demand.  
One participant can request finality at any given time.  
Other participants that received the initial request, then collaborate and sign the checkpoint.

## Finality
Finality means a certain checkpoint is agreed upon and no events can be inserted by the conflict resolution algorithm before it.

## Trust
Finality is cryptographically verifiable once reached, and can therefore be trusted to be immutable among CO participants.

## Optional
Consensus algorithms in a [CO](../reference/co.md) are completely optional.  
The [Log](./log.md) with the [Merkle-CRDT](../glossary/glossary.md#merkle-crdt) and [Cores](../reference/core.md) solve the technical part of syncing, conflict handling, and validation.  
If no other mechanism is applied (proof of authority, etc.), then the Merkle-CRDT serves as a single source of truth.  
Depending on your project requirements, you can implement any other consensus mechanism.

## Asynchronous
In CO-kit, consensus is asynchronous and on-demand, meaning users only have to wait for it, if it is really needed.  
While conventional databases normally provide finality by default (i.e. when a change is written/committed, it is final), this comes at the cost of every change losing time by having a lot of overhead (latency, routing, compute, ...)

## Proof of Authority
Proof of Authority (PoA) is the built-in consensus mechanism used in CO-kit.

It is implemented in the [`co-core-poa`](/crate/co_core_poa/index.html) Core.

When the creator of a CO adds this Core, the authority can be specified as a list of DIDs.
This authority is then responsible for voting — and once a majority is reached on a checkpoint, it is finalized.
The PoA allows for Byzantine Fault Tolerance (BFT) when configured with a majority of at least two-thirds of the authority.

By default `2/3` majority is used.

For further information, see:
- [Glossary: Proof of Authority](../glossary/glossary.md#proof-of-authority)
- [co-core-poa](../reference/core.md#co-core-poa)

## Config
CO-kit allows you to choose the level of coordination needed for each CO:
- `none`: No explicit agreement rules - relies solely on CRDT merge guarantees.
- `proof-of-authority`: Updates must be approved or signed by designated participants.
- `manual`[^issue-87]: Users or admins explicitly approve changes before commit.
- `shared`[^issue-88]: A quorum or team of peers must agree to apply state changes.

### Use Cases
With this mechanism, new and conventional use cases can be supported.

#### Client / Server
Just define the Server (or Servers) as the authority.  
Clients can be sure that the finalized state is known by the server.

#### Receipts
When two parties buy/sell something from each other, a CO can be used as a receipt between them.  
After all data has been captured, both can sign and therefore finalize the CO and receive a verifiable proof that a transaction took place.

#### Multi-Region Cloud Service
When collaborating globally, even across continents (e.g. Europe, North America, Asia), the majority can be formed by two of the continents, thus making sure a change has been globally replicated/accepted.

## See also
- [co-core-poa](../reference/core.md#co-core-poa)
- [Guards](../reference/guards.md)


[^issue-87]: [Consensus: Manual (#87)](https://gitlab.1io.com/1io/co-sdk/-/issues/87)
[^issue-88]: [Consensus: Public (#88)](https://gitlab.1io.com/1io/co-sdk/-/issues/88)