use crate::*;
use std::collections::{hash_map::Entry, HashMap};

ghost_actor::ghost_actor! {
    /// The top-level mud container - holds all rooms
    pub actor World<MudError> {
        fn room_get(room_key: RoomKey) -> RoomSender;
    }
}

pub async fn spawn_world() -> WorldSender {
    let (sender, driver) = WorldSender::ghost_actor_spawn(|i_s| {
        async move { Ok(WorldImpl::new(i_s)) }.must_box()
    })
    .await
    .unwrap();

    tokio::task::spawn(driver);

    sender
}

enum MaybeRoom {
    Pending(Vec<futures::channel::oneshot::Sender<RoomSender>>),
    Exists(RoomSender),
}

impl MaybeRoom {
    fn get_room_fut(&mut self) -> WorldHandlerResult<RoomSender> {
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

    fn set(&mut self, room: RoomSender) {
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
    internal_sender: WorldInternalSender<WorldInner>,
    rooms: HashMap<RoomKey, MaybeRoom>,
}

impl WorldImpl {
    pub fn new(internal_sender: WorldInternalSender<WorldInner>) -> Self {
        Self {
            internal_sender,
            rooms: HashMap::new(),
        }
    }
}

impl WorldHandler<(), WorldInner> for WorldImpl {
    fn handle_room_get(
        &mut self,
        room_key: RoomKey,
    ) -> WorldHandlerResult<RoomSender> {
        let maybe_room = match self.rooms.entry(room_key.clone()) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => {
                let mut i_s = self.internal_sender.clone();
                tokio::task::spawn(async move {
                    let room =
                        spawn_room(i_s.clone().into(), room_key.clone()).await;
                    i_s.ghost_actor_internal()
                        .room_add(room_key, room)
                        .await
                        .unwrap();
                });
                e.insert(MaybeRoom::Pending(Vec::new()))
            }
        };

        maybe_room.get_room_fut()
    }

    fn handle_ghost_actor_internal(
        &mut self,
        input: WorldInner,
    ) -> WorldResult<()> {
        tokio::task::spawn(input.dispatch(self));
        Ok(())
    }
}

ghost_actor::ghost_chan! {
    chan WorldInner<MudError> {
        fn room_add(room_key: RoomKey, room: RoomSender) -> ();
    }
}

impl WorldInnerHandler for WorldImpl {
    fn handle_room_add(
        &mut self,
        room_key: RoomKey,
        room: RoomSender,
    ) -> WorldInnerHandlerResult<()> {
        self.rooms.get_mut(&room_key).unwrap().set(room);
        Ok(async move { Ok(()) }.must_box())
    }
}
