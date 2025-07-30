# core
A core (CO reducer) is a piece of data that acts like a state. Cores can be directly added to COs and they work like an in-code database. They implement a reducer function that take actions which have been pushed to a CO. The reducer then changes the cores data accordingly.

### Technical Notes
- A core can be easily serialized (we use cbor) and saved
- Serialization yields a CID which can then be used to reference that data (via a log or in other states for example)
- The serialized data can then be stored on the hard drive or sent to other participants

### 1io cores
#todo 
We provide a set of cores. They are the following:
- message core
- etc...