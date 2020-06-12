use crate::*;

pub type RoomKey = (i32, i32, i32);

pub enum Dir {
    North,
    South,
    East,
    West,
    Up,
    Down,
}

impl Dir {
    fn translate_room_key(&self, room_key: &RoomKey) -> RoomKey {
        let x = room_key.0;
        let y = room_key.1;
        let z = room_key.2;
        match self {
            Self::North => y + 1,
            Self::South => y - 1,
            Self::East => x + 1,
            Self::West => x - 1,
            Self::Up => z + 1,
            Self::Down => z - 1,
        };
        (x, y, z)
    }
}

ghost_actor::ghost_actor! {
    /// A room exists in a world and holds entities.
    pub actor Room<MudError> {
        fn room_key_get() -> RoomKey;
        fn room_name_set(name: String) -> ();
        fn room_name_get() -> String;
        fn look(dir: Dir) -> String;
        fn entity_hold(entity: EntitySender) -> ();
        fn entity_drop(entity: EntitySender) -> ();
    }
}

pub async fn spawn_room(world: WorldSender, room_key: RoomKey) -> RoomSender {
    let (sender, driver) = RoomSender::ghost_actor_spawn(|_i_s| {
        async move { Ok(RoomImpl::new(world, room_key)) }.must_box()
    })
    .await
    .unwrap();

    tokio::task::spawn(driver);

    sender
}

struct RoomImpl {
    world: WorldSender,
    room_key: RoomKey,
    name: String,
    entities: Vec<EntitySender>,
}

impl RoomImpl {
    pub fn new(world: WorldSender, room_key: RoomKey) -> Self {
        Self {
            world,
            room_key,
            name: "[no-name]".to_string(),
            entities: Vec::new(),
        }
    }
}

impl RoomHandler<(), ()> for RoomImpl {
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

    fn handle_look(&mut self, dir: Dir) -> RoomHandlerResult<String> {
        let look_room = dir.translate_room_key(&self.room_key);

        let mut world = self.world.clone();
        Ok(async move {
            let mut look_room = world.room_get(look_room).await?;
            let room_name = look_room.room_name_get().await?;
            Ok(format!("You see {}", room_name))
        }
        .must_box())
    }

    fn handle_entity_hold(&mut self, mut entity: EntitySender) -> RoomHandlerResult<()> {
        self.entities.push(entity.clone());
        let room_key = self.room_key.clone();

        Ok(async move {
            entity.room_set(room_key).await?;
            Ok(())
        }.must_box())
    }
}
