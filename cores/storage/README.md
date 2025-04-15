# Co Storage Core

## Abstract

Stores informations about blocks that actually exists on disk. This COre is usually only used on local device.
The main purpose is to store pinning and reference counts.

## Structural References

Each block may containes structural refererences. This are references to blocks that will be also referenced wehn the block gets a reference (hierarchy).
This greatly reduces the overhead when a root has multiple pins as only the root needs to be reference counted.
Each structural refererence children in the set increments the refcount of the children by one.
