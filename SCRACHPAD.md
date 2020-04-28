# State of the implementation

- ☑️ Actor is activated on first message
- ☑️ Actor can send messages to other actors
- ☑️ System can send messages to any actor
- ☑️ Actor self stop
- ☑️ Stop waits for all actors to consume all messages
- ☑️ System statistics
- ☑️ RPC like messages between actors
- ☑️ Services (statefull or stateless, like actors, without ID and processing messages concurrently)
- ☑️ Automatic deallocation of unused actors
- ☑️ Subscribe to message
- □ Actor deallocation configuration (based in RAM, Actor count, fully manual or timeout)
- □ Allow more than 150.000 queued messages per actor (waiting for async_std to have unbounded channels: [https://github.com/async-rs/async-std/issues/212]())
- □ Add service with "unlimited" concurrency for cases where DB queries need to be done or cases where they are just the middleman between external world and actors.
- ☑️ Create an example with Tide
- □ Create big examples
- □ Create adaptors for Tide (like osome sort of Derive)
- □ Create some middleware structure (in case we want to do some action after each message process)
- □ Research how to do resilent Actors (resistent to unwind?)
- □ Develop a way to kill an actor without processing all the queued messages and send the queued messages later (kind of, this actor is broken, stop, reload the actor, continue processing)
- □ Allow to move actors from different Acteur instances
- □ Allow to have actors that should never be deallocated. 

# Notes

### Possible names for calls methods:

- send / call 
- notify / request
- publish / subscribe
- send_to_service / call_to_service - send_to_actor / call_to_actor 
