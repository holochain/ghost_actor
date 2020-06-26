#![deny(missing_docs)]
//! Example showing the "Clone Channel Factory" GhostActor pattern.
//! Facilitates an actor's ability to absorb additional channel
//! receivers post-spawn.

use ghost_actor::*;

ghost_chan! {
    /// A message stream
    pub chan Message<GhostError> {
        /// Send a message.
        fn message(message: String) -> ();
    }
}

/// A message receiver type
pub type MessageReceiver = futures::channel::mpsc::Receiver<Message>;

ghost_chan! {
    /// A parent actor to show off absorbing receivers
    pub chan Parent<GhostError> {
        /// Start handling messages post-spawn
        fn attach_message_receiver(r: MessageReceiver) -> ();
    }
}

/// Construct our parent actor implementation.
pub async fn spawn_parent() -> GhostSender<Parent> {
    let builder = actor_builder::GhostActorBuilder::new();

    // we can clone the channel factory!
    let channel_factory = builder.channel_factory().clone();

    // create our parent sender
    let sender = channel_factory.create_channel::<Parent>().await.unwrap();

    // we can accept the channel factory as a state item in our handler impl!
    tokio::task::spawn(builder.spawn(ParentImpl { channel_factory }));

    // return the parent sender
    sender
}

// -- private -- //

/// This is our implementation/handler/actor struct.
struct ParentImpl {
    // this is the type of the channel_factory
    // note it takes the type of Handler it's creating channels for
    // and that type is THIS TYPE - i.e. `Self`
    channel_factory: actor_builder::GhostActorChannelFactory<Self>,
}

impl GhostControlHandler for ParentImpl {}

impl GhostHandler<Message> for ParentImpl {}

impl MessageHandler for ParentImpl {
    fn handle_message(&mut self, message: String) -> MessageHandlerResult<()> {
        println!("received message: {}", message);
        Ok(must_future::MustBoxFuture::new(async move { Ok(()) }))
    }
}

impl GhostHandler<Parent> for ParentImpl {}

impl ParentHandler for ParentImpl {
    fn handle_attach_message_receiver(
        &mut self,
        r: MessageReceiver,
    ) -> ParentHandlerResult<()> {
        let fut = self.channel_factory.attach_receiver(r);
        Ok(must_future::MustBoxFuture::new(async move {
            fut.await?;
            Ok(())
        }))
    }
}

#[tokio::main]
async fn main() {
    let parent_sender = spawn_parent().await;

    // for whatever reason, our message receivers aren't ready
    // before we spawn the parent, so we have to attach them after.
    let (send1, recv1) = futures::channel::mpsc::channel(10);
    let (send2, recv2) = futures::channel::mpsc::channel(10);

    parent_sender.attach_message_receiver(recv1).await.unwrap();
    parent_sender.attach_message_receiver(recv2).await.unwrap();

    send1.message("test message 1".to_string()).await.unwrap();
    send2.message("test message 2".to_string()).await.unwrap();
}
