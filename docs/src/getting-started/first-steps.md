# First Steps
Before we start working with COkit, we'll first look at some basic concepts, and sketch out a few cases to demonstrate how it is useful.

## Introducing CO
A [CO](../reference/co.md) is a virtual room for collaboration.  
CO stands for Collaborative Object. It allows participants to access and modify the Cores contained within it.

A CO may contain:
- one or more Cores
- Participants (i.e. who can access the CO)
- Network Settings (i.e. connectivity configuration)
- Encryption Settings (i.e. encrypted or public)

## Introducing Cores
[Core](../reference/core.md) stands for CO Reducer, and it is a data model used within a CO. Cores model data, business logic and permissions.  

Being a reducer, a Core takes a state and an action as inputs, calculates how the state will change based on that action, and returns the new state.

Here is an example data model of a to-do list task in a Core:

```rust
/// A to-do list task.
#[co]
pub struct TodoTask {
	/// Task UUID.
	pub id: String,
	/// Task title.
	pub title: String,
	/// Whether the task is done.
	pub done: bool,
}
```

## Use Case: Collaborative to-do list
A simple example of how to use COkit is a collaborative to-do list. This is what we will build in the Quick-Start sections of this documentation.

The Core Quick Start covers the fundamentals of using COkit, while also demonstrating just how easy it is to create and use a collaborative Core.

The App Quick Starts are for those who want some guidance in developing a simple app to use the to-do list Core.
