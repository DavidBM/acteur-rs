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
- □ Create adapters for Tide (like some sort of Derive)
- □ Create some middle-ware structure (in case we want to do some action after each message process)
- □ Research how to do resilient Actors (resistant to unwind?)
- □ Develop a way to kill an actor without processing all the queued messages and send the queued messages later (kind of, this actor is broken, stop, reload the actor, continue processing)
- □ Allow to move actors from different Acteur instances
- □ Allow to have actors that should never be deallocated. 

# Notes

### Possible names for calls methods:

- send / call 
- notify / request
- publish / subscribe
- send_to_service / call_to_service - send_to_actor / call_to_actor 

# Concurrency plan

The idea is to have a raft in each node keeping a log of where is each actor-id and each service. I need to check if the service can be handled by the typeId in different platforms.

Given that we have the actor Proxy and manager, we can have manager checking where is each Actor and routing the message internally or sending it for network dispatch.

The framework should take special care of not loosing messages if, for example, a system crashes. In such case the messages that were enqueued to be delivered to the node, should be delivered to the node taking the actors.

If a nodes becomes too busy, it should be able to send actors to other systems.

For services, they can be marked as to be in several system sin the same way that kubernetes allows to configure how many replicas should have the service.

The message ordering for a same actor instance should be kept even in the network case. Meaning, no case should happen where a node sends a message to two different nodes for the same actor and they happen in a different order. Still, the transaction boundary here is a bit diffused. What should be considered a "must keep the order" and what no? Does a browser sending two HTTP requests to two different nodes considered a transaction? 

I think that in the case or external communication, the framework should explain how it works internally for the developer to tale care of not sending each message to a random node.
