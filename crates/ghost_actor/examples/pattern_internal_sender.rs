#![deny(missing_docs)]
//! Example showing the "Internal Sender" GhostActor pattern.
//! Facilitates undertaking async work in GhostActor handler functions.

use ghost_actor::*;

/// A helper struct to illustrate our results.
#[derive(Debug)]
pub struct Response {
    start_index: u32,
    finish_index: u32,
}

ghost_chan! {
    /// Public Actor Api - This generates our "External" Sender.
    pub chan MyActorApi<GhostError> {
        /// An API function to call.
        fn my_api() -> Response;
    }
}

/// Construct our MyActorApi implementation.
pub async fn spawn_my_impl() -> GhostSender<MyActorApi> {
    let builder = actor_builder::GhostActorBuilder::new();

    let internal_sender = builder
        .channel_factory()
        .create_channel::<MyInternalApi>()
        .await
        .unwrap();

    let sender = builder
        .channel_factory()
        .create_channel::<MyActorApi>()
        .await
        .unwrap();

    tokio::task::spawn(builder.spawn(MyImpl {
        index: 0,
        internal_sender,
    }));

    sender
}

// -- private -- //

ghost_chan! {
    /// Internal Api - This generates our "Internal" Sender.
    chan MyInternalApi<GhostError> {
        /// Internal api function to call.
        fn finalize_api(start_index: u32) -> Response;
    }
}

/// This is our implementation/handler/actor struct.
struct MyImpl {
    /// represents some internal state that may be modified.
    index: u32,
    /// this is the sender handle to our "Internal Sender".
    internal_sender: GhostSender<MyInternalApi>,
}

impl MyImpl {
    /// Mutate our internal state.
    fn next_index(&mut self) -> u32 {
        self.index += 1;
        self.index
    }
}

impl GhostControlHandler for MyImpl {}

impl GhostHandler<MyActorApi> for MyImpl {}

const DELAY_TIME: &'static [u64] = &[3, 2, 5, 1, 4];

impl MyActorApiHandler for MyImpl {
    // implement our external api handler
    fn handle_my_api(&mut self) -> MyActorApiHandlerResult<Response> {
        let start_index = self.next_index();
        let i_s = self.internal_sender.clone();
        Ok(must_future::MustBoxFuture::new(async move {
            // simulate differing amounts of work
            // that can be undertaken in parallel
            // without involving `&mut self`
            tokio::time::sleep(std::time::Duration::from_millis(
                DELAY_TIME[(start_index % 5) as usize],
            ))
            .await;

            // now that this parallel work is done
            // presumably we'll need to adjust our internal state
            // thus, we make use of the internal_sender pattern:
            i_s.finalize_api(start_index).await
        }))
    }
}

impl GhostHandler<MyInternalApi> for MyImpl {}

impl MyInternalApiHandler for MyImpl {
    // implement our internal api handler
    fn handle_finalize_api(
        &mut self,
        start_index: u32,
    ) -> MyInternalApiHandlerResult<Response> {
        // parallel work is done, adjust our internal state:
        let finish_index = self.next_index();
        Ok(must_future::MustBoxFuture::new(async move {
            Ok(Response {
                start_index,
                finish_index,
            })
        }))
    }
}

#[tokio::main]
async fn main() {
    // spawn our actor
    let actor = spawn_my_impl().await;

    // execute 10 api calls in parallel
    let res = futures::future::join_all(
        (0..10).map(|_| actor.my_api()).collect::<Vec<_>>(),
    )
    .await;

    // printing out the results, we should see that the
    // "start_index"es likely come out in sequence, while
    // the "finish_index"es in a different order depending
    // on the internal delays in the api functions.
    println!("{:#?}", res);
}
