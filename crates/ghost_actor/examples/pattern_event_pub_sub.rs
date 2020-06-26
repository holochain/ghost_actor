#![deny(missing_docs)]
//! Example showing the "Event Publish/Subscribe" GhostActor pattern.
//! Facilitates an actor's ability to async emit notifications/requests,
//! and a "parent" actor being able to handle events from a child actor.

use ghost_actor::*;

ghost_chan! {
    /// An event emitted by a "TickActor".
    pub chan TickEvent<GhostError> {
        /// A "tick" event with a message.
        fn tick(message: String) -> ();
    }
}

/// Channel receiver for "TickEvent" messages.
pub type TickEventReceiver = futures::channel::mpsc::Receiver<TickEvent>;

ghost_chan! {
    /// An actor that emits "tick" events.
    pub chan TickActor<GhostError> {
        /// Begin a new tick timer that will send a message every
        /// "interval_ms" milliseconds.
        fn start_tick(prefix: String, interval_ms: u64) -> ();
    }
}

/// Construct our MyActorApi implementation.
pub async fn spawn_tick() -> (GhostSender<TickActor>, TickEventReceiver) {
    let builder = actor_builder::GhostActorBuilder::new();

    // we can manually create channels with GhostActor event types
    let (event_sender, event_receiver) = futures::channel::mpsc::channel(10);

    let sender = builder
        .channel_factory()
        .create_channel::<TickActor>()
        .await
        .unwrap();

    // track the sender inside our handler so we can send to it
    tokio::task::spawn(builder.spawn(TickImpl { event_sender }));

    // return both
    // - the tick sender for setting up new tick loops
    // - the event receiver so our parent can receive events
    (sender, event_receiver)
}

// -- private -- //

/// This is our implementation/handler/actor struct.
struct TickImpl {
    event_sender: futures::channel::mpsc::Sender<TickEvent>,
}

impl GhostControlHandler for TickImpl {}

impl GhostHandler<TickActor> for TickImpl {}

impl TickActorHandler for TickImpl {
    fn handle_start_tick(
        &mut self,
        prefix: String,
        interval_ms: u64,
    ) -> TickActorHandlerResult<()> {
        let event_sender = self.event_sender.clone();
        tokio::task::spawn(async move {
            loop {
                if let Err(_) = event_sender
                    .tick(format!("{} - {} ms tick", prefix, interval_ms))
                    .await
                {
                    break;
                }

                tokio::time::delay_for(std::time::Duration::from_millis(
                    interval_ms,
                ))
                .await;
            }
        });
        Ok(must_future::MustBoxFuture::new(async move { Ok(()) }))
    }
}

#[tokio::main]
async fn main() {
    // create our tick actor
    let (tick_sender, tick_receiver) = spawn_tick().await;
    tick_sender
        .start_tick("Apple".to_string(), 1)
        .await
        .unwrap();
    tick_sender
        .start_tick("Banana".to_string(), 9)
        .await
        .unwrap();

    // inline "Parent" actor handler
    struct ParentImpl;
    impl GhostControlHandler for ParentImpl {}
    impl GhostHandler<TickEvent> for ParentImpl {}
    impl TickEventHandler for ParentImpl {
        fn handle_tick(
            &mut self,
            message: String,
        ) -> TickEventHandlerResult<()> {
            println!("got tick: {}", message);
            Ok(must_future::MustBoxFuture::new(async move { Ok(()) }))
        }
    }

    // create a "Parent" actor
    let builder = actor_builder::GhostActorBuilder::new();

    // attach the receiver we got from creating the tick actor
    // NOTICE: we're using `attach_receiver` instead of `create_channel` here.
    builder
        .channel_factory()
        .attach_receiver(tick_receiver)
        .await
        .unwrap();

    // spawn our parent actor
    tokio::task::spawn(builder.spawn(ParentImpl));

    // wait for a bit to see tick events happen
    tokio::time::delay_for(std::time::Duration::from_millis(20)).await;
}
