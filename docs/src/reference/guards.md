# Guards
Guards are checks for transactions.  
They serve as a sort of "police" for transactions, deciding which transactions make it into the [Log](../reference/log.md) and which do not.

New transactions will be checked by the configured Guards of a CO and will be rejected if not all Guards succeed.  
Just like [Cores](../reference/core.md), Guards are pure functions, are compiled to WebAssembly, and are registered to COs.

This mechanism is used as the basis for implementing consensus algorithms and checks that are true for every transaction in a CO.

```admonish tip
Guards are not [Permissions](../reference/permissions.md).

Guards should be designed to return the same result independent of the order of the transactions.

Permissions are order-dependent.
```

Technically:  
Guards reject transactions from a peer immediately, before those transactions reach the conflict resolution of the [CRDT](../glossary/glossary.md#crdt).  
Therefore, transactions rejected by Guards will not make it to the [CO](../reference/co.md), even if they would become valid when other peers join transactions.

## Built-in Guards
### Check: Is Participant
The simplest Guard is that a peer must be a participant in order to write transactions to the [CO](../reference/co.md).

This Guard is implemented in [`co-core-co`](/crate/co_core_co/struct.Co.html#impl-Guard<S>-for-Co).

### Check: POA Consensus Conformance
The Proof-of-Authority Consensus mechanism checks new transactions for conformance to the latest reached consensus, and will reject the transaction if it does not conform.  

Technically:  
the Guard accesses the state of the [Proof-of-Authority Core](../reference/core.md#co-core-poa) and checks the transaction against it.

#### Diagram: How POA Consensus works internally
This sequence shows how Guards process new transactions received from the network:

```mermaid
sequenceDiagram
participant N as Network
participant C as CO
participant G as Guard
participant P as POA Core
participant L as Log
N ->> C: Heads
C ->> C: Resolve Guards
loop Guards
	C ->> G: Check transaction
	G ->> P: Read consensus
	P --> G: Latest consensus
	alt Check conformance
		G --> C: Ok
	else
		G --x C: Reject
	end
end
C ->> L: Join Heads
L --> C: New CO heads
opt Different new heads
	C ->> N: Send new heads
end
```
