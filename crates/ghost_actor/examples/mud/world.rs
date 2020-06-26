use crate::*;
use std::collections::{hash_map::Entry, HashMap};

ghost_actor::ghost_chan! {
    /// The top-level mud container - holds all rooms
    pub chan World<MudError> {
        fn room_get(room_key: RoomKey) -> ghost_actor::GhostSender<Room>;
    }
}

pub async fn spawn_world() -> ghost_actor::GhostSender<World> {
    let builder = ghost_actor::actor_builder::GhostActorBuilder::new();

    let i_s = builder
        .channel_factory()
        .create_channel::<WorldInner>()
        .await
        .unwrap();

    let sender = builder
        .channel_factory()
        .create_channel::<World>()
        .await
        .unwrap();

    tokio::task::spawn(builder.spawn(WorldImpl::new(sender.clone(), i_s)));

    sender
}

enum MaybeRoom {
    Pending(
        Vec<futures::channel::oneshot::Sender<ghost_actor::GhostSender<Room>>>,
    ),
    Exists(ghost_actor::GhostSender<Room>),
}

impl MaybeRoom {
    fn get_room_fut(
        &mut self,
    ) -> WorldHandlerResult<ghost_actor::GhostSender<Room>> {
        match self {
            Self::Pending(pending) => {
                let (s, r) = futures::channel::oneshot::channel();
                pending.push(s);
                Ok(async move { Ok(r.await?) }.must_box())
            }
            Self::Exists(room) => {
                let room = room.clone();
                Ok(async move { Ok(room) }.must_box())
            }
        }
    }

    fn set(&mut self, room: ghost_actor::GhostSender<Room>) {
        let prev = std::mem::replace(self, Self::Exists(room.clone()));
        match prev {
            Self::Pending(pending) => {
                for p in pending {
                    let _ = p.send(room.clone());
                }
            }
            _ => panic!("MaybeRoom::set called twice"),
        }
    }
}

struct WorldImpl {
    external_sender: ghost_actor::GhostSender<World>,
    internal_sender: ghost_actor::GhostSender<WorldInner>,
    rooms: HashMap<RoomKey, MaybeRoom>,
}

impl WorldImpl {
    pub fn new(
        external_sender: ghost_actor::GhostSender<World>,
        internal_sender: ghost_actor::GhostSender<WorldInner>,
    ) -> Self {
        Self {
            external_sender,
            internal_sender,
            rooms: HashMap::new(),
        }
    }
}

impl ghost_actor::GhostControlHandler for WorldImpl {}

impl ghost_actor::GhostHandler<World> for WorldImpl {}

impl WorldHandler for WorldImpl {
    fn handle_room_get(
        &mut self,
        room_key: RoomKey,
    ) -> WorldHandlerResult<ghost_actor::GhostSender<Room>> {
        let maybe_room = match self.rooms.entry(room_key.clone()) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => {
                let x_s = self.external_sender.clone();
                let i_s = self.internal_sender.clone();
                tokio::task::spawn(async move {
                    let room = spawn_room(x_s, room_key.clone()).await;
                    i_s.room_add(room_key, room).await.unwrap();
                });
                e.insert(MaybeRoom::Pending(Vec::new()))
            }
        };

        maybe_room.get_room_fut()
    }

    /*
    fn handle_ghost_actor_internal(
        &mut self,
        input: WorldInner,
    ) -> WorldResult<()> {
        tokio::task::spawn(input.dispatch(self));
        Ok(())
    }
    */
}

ghost_actor::ghost_chan! {
    chan WorldInner<MudError> {
        fn room_add(room_key: RoomKey, room: ghost_actor::GhostSender<Room>) -> ();
    }
}

impl ghost_actor::GhostHandler<WorldInner> for WorldImpl {}

impl WorldInnerHandler for WorldImpl {
    fn handle_room_add(
        &mut self,
        room_key: RoomKey,
        room: ghost_actor::GhostSender<Room>,
    ) -> WorldInnerHandlerResult<()> {
        self.rooms.get_mut(&room_key).unwrap().set(room);
        Ok(async move { Ok(()) }.must_box())
    }
}
