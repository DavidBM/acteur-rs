## Actor ending process [TODO]

The process of ending an actor should be synchronous in order to lower the change to a race condition. At the same time, ActorProxie should have an state (ending, running, etc). 

If a message needs to be sent to a ending ActorProxy, the actor proxy should return a future that will be fulfilled once the last message is being sent. In that way, we ensure we are not creating a new actor with the same ID which may consume messages that should be processed after the messages of the ending actorProxy.

- Same for ActorsManager

The .end method should return a promise that fulfills only when the actor/actorsManager has no more messages/actors

End process:

Sync  | Call SystemDirector::end_actor
Sync  | Call ActorsManager::end_actor
Sync  | Call ActorProxy::end
Sync  | ActorProxy sends the END message into the actors proxy
Sync  | ActorsManager marks that ActorProxy as ending
... If a new message arrives, all good
Sync  | ActorProxy gets END message
Sync  | ActorProxy calls ActorsManager remove actor (maybe through SystemDirector). 
    Sync  | If ActorsManager has 0 actors and 0 messages call SystemDirector::remove_manager
        TODO: Check what to do if there are messages, provably nothing, but that means that we need to check at every last message if we have remaining actors
Sync  | ActorProxy checks the queue for new messages. If messages, sends them via the SystemDirector.
Sync  | Breaks the loop

We can do this ir just reschedule the END message
