# Log
The Log is a conflict-free replicated event stream which is immutable and cryptographically verifiable.
It is (eventually consistent) sorted using a Merkle-DAG-based logical clock.
Arbitrary heads can be joined together at any time.
Whenever the same heads are joined the resulting log is guaranteed to be equal.

## What makes a Log
This can be thought of like a git graph where each commit is an operation.
The heads represent the end of the log and also a specific state of the data.

## How it is used in CO-kit
Each [CO](./co.md) is event-sourced by a Log.
The CO state is materialized from the log through its set of cores.
The Log is implemented in the `co-log` project.

## References
- https://arxiv.org/abs/2004.00107

## Example
This example shows how sorting works with sample data.
In this example the number represents the logical clock.
1. Illustrating a graph with three participants.
2. The resulting sorted list without the heads `9'` and thus `C5'`.
3. The resulting sorted list with the heads `9'` and thus `C5'`.

Whenever there is a causal "conflict" we got two or more heads for a logical clock.

### 1. Graph
```mermaid
block-beta
columns 3
	Alice
	Bob
	Charlie
	space 1     space
	space:3
	space space 2
	space:3
	space 3     space
	space:3
	4     space space
	space:3
	A5    space C5["C5'"]
	space:3
	A6    B6    space
	space:3
	7     space space
	space:3
	space 8     space
	space:3
	9["9'"] space space

	2 --> 1
	3 --> 2
	4 --> 3
	A5 --> 4
	C5 --> 4
	A6 --> A5
	B6 --> A5
	7 --> A6
	7 --> B6
	8 --> 7
	9 --> 8
	9 --> C5

	style Alice stroke-width:4px
	style Bob stroke-width:4px
	style Charlie stroke-width:4px

	style 9 fill:#faa
	style C5 fill:#faa
```

### 2. Sequence before `'`

```mermaid
block-beta
columns 1
	1
	space
	2
	space
	3
	space
	4
	space
	A5
	space
	A6
	space
	B6
	space
	7
	space
	8

	2 --> 1
	3 --> 2
	4 --> 3
	A5 --> 4
	A6 --> A5
	B6 --> A5
	7 --> A6
	7 --> B6
	8 --> 7
```


### 3. Sequence after  `'`

```mermaid
block-beta
columns 1
	1
	space
	2
	space
	3
	space
	4
	space
	A5
	space
	C5["C5"]
	space
	A6
	space
	B6
	space
	7
	space
	8
	space
	9["9'"]

	2 --> 1
	3 --> 2
	4 --> 3
	A5 --> 4
	A6 --> A5
	B6 --> A5
	7 --> A6
	7 --> B6
	8 --> 7
	9 --> 8
	9 --> C5

	style 9 fill:#faa
	style C5 fill:#faa
```
