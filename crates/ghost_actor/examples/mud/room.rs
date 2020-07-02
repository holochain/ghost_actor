use crate::*;
use std::collections::HashSet;

pub type RoomKey = (i32, i32, i32);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Direction {
    North,
    South,
    East,
    West,
    Up,
    Down,
}

impl Direction {
    pub fn translate_room_key(&self, room_key: &RoomKey) -> RoomKey {
        let (mut x, mut y, mut z) = room_key.clone();
        match self {
            Self::North => y += 1,
            Self::South => y -= 1,
            Self::East => x += 1,
            Self::West => x -= 1,
            Self::Up => z += 1,
            Self::Down => z -= 1,
        };
        (x, y, z)
    }
}

ghost_actor::ghost_chan! {
    /// A room exists in a world and holds entities.
    pub chan Room<MudError> {
        fn room_key_get() -> RoomKey;
        fn room_name_set(name: String) -> ();
        fn room_name_get() -> String;
        fn room_exit_toggle(dir: Direction) -> ();
        fn room_has_exit(dir: Direction) -> bool;
        fn look() -> String;
        fn say(msg: String) -> ();
        fn entity_hold(entity: ghost_actor::GhostSender<Entity>) -> ();
        fn entity_drop(entity: ghost_actor::GhostSender<Entity>) -> ();
    }
}

pub async fn spawn_room(
    world: ghost_actor::GhostSender<World>,
    room_key: RoomKey,
) -> ghost_actor::GhostSender<Room> {
    let builder = ghost_actor::actor_builder::GhostActorBuilder::new();

    let sender = builder
        .channel_factory()
        .create_channel::<Room>()
        .await
        .unwrap();

    tokio::task::spawn(builder.spawn(RoomImpl::new(
        sender.clone(),
        world,
        room_key,
    )));

    sender
}

struct RoomImpl {
    external_sender: ghost_actor::GhostSender<Room>,
    #[allow(dead_code)]
    world: ghost_actor::GhostSender<World>,
    room_key: RoomKey,
    name: String,
    entities: HashSet<ghost_actor::GhostSender<Entity>>,
    exits: HashSet<Direction>,
}

impl RoomImpl {
    pub fn new(
        external_sender: ghost_actor::GhostSender<Room>,
        world: ghost_actor::GhostSender<World>,
        room_key: RoomKey,
    ) -> Self {
        Self {
            external_sender,
            world,
            room_key,
            name: "[no-name]".to_string(),
            entities: HashSet::new(),
            exits: HashSet::new(),
        }
    }
}

impl ghost_actor::GhostControlHandler for RoomImpl {}

impl ghost_actor::GhostHandler<Room> for RoomImpl {}

impl RoomHandler for RoomImpl {
    fn handle_room_key_get(&mut self) -> RoomHandlerResult<RoomKey> {
        let room_key = self.room_key.clone();

        Ok(async move { Ok(room_key) }.must_box())
    }

    fn handle_room_name_set(&mut self, name: String) -> RoomHandlerResult<()> {
        self.name = name;

        Ok(async move { Ok(()) }.must_box())
    }

    fn handle_room_name_get(&mut self) -> RoomHandlerResult<String> {
        let name = self.name.clone();

        Ok(async move { Ok(name) }.must_box())
    }

    fn handle_room_exit_toggle(
        &mut self,
        dir: Direction,
    ) -> RoomHandlerResult<()> {
        if self.exits.contains(&dir) {
            self.exits.remove(&dir);
        } else {
            self.exits.insert(dir);
        }
        Ok(async move { Ok(()) }.must_box())
    }
    fn handle_room_has_exit(
        &mut self,
        dir: Direction,
    ) -> RoomHandlerResult<bool> {
        let result = self.exits.contains(&dir);
        Ok(async move { Ok(result) }.must_box())
    }

    fn handle_look(&mut self) -> RoomHandlerResult<String> {
        let msg_loc =
            format!("You are in [{}]. {:?}", self.name, self.room_key);

        let exits = self.exits.iter().collect::<Vec<_>>();
        let msg_exits = format!("Exits: {:?}", exits);

        let entities = self.entities.iter().cloned().collect::<Vec<_>>();
        Ok(async move {
            let f = futures::future::join_all(
                entities.into_iter().map(|e| e.entity_name_get()),
            );
            let msg = format!(
                "{}\r\n{}\r\nWith: {:?}",
                msg_loc,
                msg_exits,
                f.await.into_iter().map(|r| r.unwrap()).collect::<Vec<_>>(),
            );
            Ok(msg)
        }
        .must_box())
    }

    fn handle_say(&mut self, msg: String) -> RoomHandlerResult<()> {
        let entities = self.entities.iter().cloned().collect::<Vec<_>>();
        Ok(async move {
            for e in entities {
                let _ = e.say(msg.clone()).await;
            }
            Ok(())
        }
        .must_box())
    }

    fn handle_entity_hold(
        &mut self,
        entity: ghost_actor::GhostSender<Entity>,
    ) -> RoomHandlerResult<()> {
        self.entities.insert(entity.clone());
        let room = self.external_sender.clone();
        let room_key = self.room_key.clone();

        Ok(async move {
            entity.room_set(room_key, room).await?;
            Ok(())
        }
        .must_box())
    }

    fn handle_entity_drop(
        &mut self,
        entity: ghost_actor::GhostSender<Entity>,
    ) -> RoomHandlerResult<()> {
        self.entities.remove(&entity);
        Ok(async move { Ok(()) }.must_box())
    }
}
