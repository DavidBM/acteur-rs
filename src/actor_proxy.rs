use futures::task::AtomicWaker;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

use crate::envelope::{Envelope, Letter};
use crate::system_director::SystemDirector;
use crate::{Actor, Assistant, Handle};
use async_std::{
    sync::{channel, Receiver, Sender},
    task,
};
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
    // This should be a waker set
    on_end_waker: Arc<AtomicWaker>,
}

impl<A: Actor> ActorProxy<A> {
    pub fn new(system_director: SystemDirector, id: A::Id) -> ActorProxy<A> {
        let (sender, receiver): (Sender<ActorProxyCommand<A>>, Receiver<ActorProxyCommand<A>>) =
            channel(5);

        let assistant = Assistant::new(system_director, id.clone());
        let on_end_waker = Arc::new(AtomicWaker::new());

        actor_loop(
            id,
            sender.clone(),
            receiver,
            assistant,
            on_end_waker.clone(),
        );

        ActorProxy {
            sender,
            last_sent_message_time: SystemTime::now(),
            on_end_waker,
        }
    }

    pub async fn send<M: 'static>(&mut self, message: M)
    where
        A: Handle<M>,
        M: Send + Debug,
    {
        self.last_sent_message_time = SystemTime::now();

        let message = Letter::<A, M>::new(message);

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

    pub fn end(&self) -> impl Future<Output = ()> + 'static {

        let sender = self.sender.clone();
        task::spawn(async move {
            sender.send(ActorProxyCommand::End).await;
        });

        WaitActorEnd::new(self.on_end_waker.clone(), self.sender.clone())
    }
}

fn actor_loop<A: Actor>(
    id: A::Id,
    sender: Sender<ActorProxyCommand<A>>,
    receiver: Receiver<ActorProxyCommand<A>>,
    assistant: Assistant<A>,
    on_end_waker: Arc<AtomicWaker>,
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
                        ActorProxyCommand::End => {
                            // We may find cases where we can have several End command in a row. In that case,
                            // we want to consume all the end command together until we find nothing or a not-end command
                            match recv_until_command_or_end!(receiver, ActorProxyCommand::End).await
                            {
                                None => {
                                    actor.deactivate().await;
                                    on_end_waker.wake();
                                    break;
                                }
                                Some(ActorProxyCommand::Dispatch(mut envelope)) => {
                                    // If there are any message left, we postpone the shutdown.
                                    sender.send(ActorProxyCommand::End).await;
                                    envelope.dispatch(&mut actor, &assistant).await
                                }
                                _ => unreachable!(),
                            }
                        }
                    }
                }
            }
        });
    });
}

pub(crate) struct WaitActorEnd<A: Actor>(Arc<AtomicWaker>, Sender<ActorProxyCommand<A>>);

impl<A: Actor> WaitActorEnd<A> {
    pub fn new(waker: Arc<AtomicWaker>, sender: Sender<ActorProxyCommand<A>>) -> WaitActorEnd<A> {
        WaitActorEnd(waker, sender)
    }
}

impl<A: Actor> Future for WaitActorEnd<A> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        if self.1.len() > 0 {
            self.0.register(cx.waker());
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}
