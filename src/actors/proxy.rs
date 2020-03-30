use crate::actors::manager::ActorsManager;
use crate::envelope::{Envelope, Letter, LetterWithResponder};
use crate::system_director::SystemDirector;
use crate::{Actor, Assistant, Receive, Respond};
use async_std::{
    sync::{channel, Receiver, Sender},
    task,
};
use dashmap::mapref::entry::Entry::Occupied;
use std::fmt::Debug;
use std::time::SystemTime;

#[derive(Debug)]
pub(crate) enum ActorProxyCommand<A: Actor> {
    Dispatch(Box<dyn Envelope<Actor = A>>),
    End,
}

pub struct ActorReport {
    pub last_message_on: SystemTime,
    pub enqueued_messages: usize,
}

#[derive(Debug)]
pub(crate) struct ActorProxy<A: Actor> {
    sender: Sender<ActorProxyCommand<A>>,
    last_sent_message_time: SystemTime,
}

impl<A: Actor> ActorProxy<A> {
    pub fn new(
        system_director: SystemDirector,
        manager: ActorsManager<A>,
        id: A::Id,
    ) -> ActorProxy<A> {
        let (sender, receiver): (Sender<ActorProxyCommand<A>>, Receiver<ActorProxyCommand<A>>) =
            channel(5);

        let assistant = Assistant::new(system_director.clone(), manager.clone(), id.clone());

        actor_loop(id, sender.clone(), receiver, assistant, manager);

        ActorProxy {
            sender,
            last_sent_message_time: SystemTime::now(),
        }
    }

    pub async fn send<M: 'static>(&mut self, message: M)
    where
        A: Receive<M>,
        M: Send + Debug,
    {
        self.last_sent_message_time = SystemTime::now();

        let message = Letter::<A, M>::new(message);

        // TODO: Handle the channel disconnection properly
        self.sender
            .send(ActorProxyCommand::Dispatch(Box::new(message)))
            .await;
    }

    pub async fn call<M: 'static>(
        &mut self,
        message: M,
        responder: Sender<<A as Respond<M>>::Response>,
    ) where
        A: Respond<M>,
        M: Send + Debug,
    {
        self.last_sent_message_time = SystemTime::now();

        let message = LetterWithResponder::<A, M>::new(message, responder);

        // TODO: Handle the channel disconnection properly
        self.sender
            .send(ActorProxyCommand::Dispatch(Box::new(message)))
            .await;
    }

    pub fn get_last_sent_message_time(&self) -> SystemTime {
        self.last_sent_message_time
    }

    pub fn get_inbox_length(&self) -> usize {
        self.sender.len()
    }

    pub fn get_report(&self) -> ActorReport {
        ActorReport {
            last_message_on: self.get_last_sent_message_time(),
            enqueued_messages: self.get_inbox_length(),
        }
    }

    pub fn end(&self) {
        let sender = self.sender.clone();
        task::spawn(async move {
            sender.send(ActorProxyCommand::End).await;
        });
    }
}

fn actor_loop<A: Actor>(
    id: A::Id,
    sender: Sender<ActorProxyCommand<A>>,
    receiver: Receiver<ActorProxyCommand<A>>,
    assistant: Assistant<A>,
    manager: ActorsManager<A>,
) {
    task::spawn(async move {
        let mut actor = A::activate(id.clone()).await;

        task::spawn(async move {
            loop {
                if let Some(command) = receiver.recv().await {
                    match command {
                        ActorProxyCommand::Dispatch(mut envelope) => {
                            envelope.dispatch(&mut actor, &assistant).await
                        }
                        // The end process is a bit complicated. We don't want that if a End message
                        // is issued at the same time that someone else is sending a message we process
                        // messages out of order, or in paralle, or not at all.
                        //
                        // Example: End is received, actor is removed from the HashMap. At the same time
                        // we are still processing the remaining messages. But during that, someone sends
                        // a new message, making the manager to create a new actor that may process
                        // this message before the remaining ones in the old queue.
                        //
                        // What we do is to use the method `get_own_slot_blocking` in order to get the
                        // Entry in the manager HashMap in order to block any new message sending as
                        // the the manager searched for the actor each time it needs to send a message.
                        //
                        // This allows the method to stop the sending to this actor, to check any remaining
                        // remaining mesages in the queue, consume them (if any), requeue them (if any) and
                        // finally removing the actor or not from the managers HashMap
                        ActorProxyCommand::End => {
                            // We may find cases where we can have several End command in a row.
                            // In that case, we want to consume all the end command together until
                            // we find nothing or a not-end command
                            match recv_until_command_or_end!(receiver, ActorProxyCommand::End).await
                            {
                                // We start the actor ending process.
                                None | Some(ActorProxyCommand::End) => {
                                    // We take the entry for this A::Id until we finish cleaning everything up.
                                    // This blocks any entry trying to get the ActorProxy in order to send messages.
                                    // Also, if any new message is sent to this actor, it will block the whole
                                    // message sending to this actor family. That is why we do it only after
                                    // checking that there are not more messages left, and therefore, reducing
                                    // the chances of blocking the whole message sending in the actor family.
                                    let entry = manager.get_blocking_actor_entry(id.clone());

                                    // We check again if there is any remainign message and, if any,
                                    // we requeue it and abort the ending.
                                    match recv_until_command_or_end!(
                                        receiver,
                                        ActorProxyCommand::End
                                    )
                                    .await
                                    {
                                        Some(ActorProxyCommand::Dispatch(mut envelope)) => {
                                            // We stop blocking the entry as we will continue receiving messages
                                            drop(entry);
                                            // We postpone the ending of the actor
                                            sender.send(ActorProxyCommand::End).await;
                                            // and process the found message
                                            envelope.dispatch(&mut actor, &assistant).await
                                        }
                                        None | Some(ActorProxyCommand::End) => {
                                            // If not messages are found, we just remove the actor from the HashMap
                                            if let Occupied(entry) = entry {
                                                entry.remove();
                                                // Signaling only when we really remove the actor.
                                                manager.signal_actor_removed().await;
                                            }

                                            // and stop the main loop
                                            break;
                                        }
                                    }
                                }
                                Some(ActorProxyCommand::Dispatch(mut envelope)) => {
                                    // If there are any message left, we postpone the shutdown.
                                    sender.send(ActorProxyCommand::End).await;
                                    // and process the found message
                                    envelope.dispatch(&mut actor, &assistant).await
                                }
                            }
                        }
                    }
                }
            }
        });
    });
}
