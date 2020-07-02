use crate::*;

ghost_actor::ghost_chan! {
    /// An entity represents a user / npc / or item that can move around.
    pub chan Entity<MudError> {
        fn say(msg: String) -> ();
        fn room_set(room_key: RoomKey, room: ghost_actor::GhostSender<Room>) -> ();
        fn entity_name_get() -> String;
    }
}

pub async fn spawn_con_entity(
    world: ghost_actor::GhostSender<World>,
    c_send: ghost_actor::GhostSender<Con>,
    c_recv: ConEventReceiver,
) -> ghost_actor::GhostSender<Entity> {
    let name = "[no-name]".to_string();
    c_send
        .write_raw(
            b"\r
Welcome to the GhostActor Mud Example!\r
You can type \"help\" and press enter for a list of commands.\r
"
            .to_vec(),
        )
        .await
        .unwrap();
    c_send
        .prompt_set(format!("{}> ", &name).into_bytes())
        .await
        .unwrap();

    let builder = ghost_actor::actor_builder::GhostActorBuilder::new();

    let sender = builder
        .channel_factory()
        .create_channel::<Entity>()
        .await
        .unwrap();

    builder
        .channel_factory()
        .attach_receiver(c_recv)
        .await
        .unwrap();

    let room = world.room_get((0, 0, 0).into()).await.unwrap();

    let mut i = ConEntityImpl::new(sender.clone(), world, room, c_send, name);
    i.handle_user_command("look".to_string())
        .unwrap()
        .await
        .unwrap();

    tokio::task::spawn(builder.spawn(i));

    sender
}

struct ConEntityImpl {
    external_sender: ghost_actor::GhostSender<Entity>,
    world: ghost_actor::GhostSender<World>,
    room: ghost_actor::GhostSender<Room>,
    cur_room: RoomKey,
    c_send: ghost_actor::GhostSender<Con>,
    name: String,
}

impl ConEntityImpl {
    pub fn new(
        external_sender: ghost_actor::GhostSender<Entity>,
        world: ghost_actor::GhostSender<World>,
        room: ghost_actor::GhostSender<Room>,
        c_send: ghost_actor::GhostSender<Con>,
        name: String,
    ) -> Self {
        Self {
            external_sender,
            world,
            room,
            cur_room: (0, 0, 0),
            c_send,
            name,
        }
    }
}

impl ghost_actor::GhostControlHandler for ConEntityImpl {
    fn handle_ghost_actor_shutdown(
        self,
    ) -> must_future::MustBoxFuture<'static, ()> {
        let ConEntityImpl {
            external_sender,
            room,
            c_send,
            ..
        } = self;
        must_future::MustBoxFuture::new(async move {
            let _ = room.entity_drop(external_sender).await;
            let _ = c_send.ghost_actor_shutdown_immediate().await;
        })
    }
}

impl ghost_actor::GhostHandler<Entity> for ConEntityImpl {}

impl EntityHandler for ConEntityImpl {
    fn handle_say(&mut self, msg: String) -> EntityHandlerResult<()> {
        let c_send = self.c_send.clone();
        Ok(async move {
            c_send.write_raw(msg.into_bytes()).await?;
            Ok(())
        }
        .must_box())
    }

    fn handle_room_set(
        &mut self,
        room_key: RoomKey,
        room: ghost_actor::GhostSender<Room>,
    ) -> EntityHandlerResult<()> {
        self.cur_room = room_key;
        self.room = room;
        Ok(async move { Ok(()) }.must_box())
    }

    fn handle_entity_name_get(&mut self) -> EntityHandlerResult<String> {
        let name = self.name.clone();
        Ok(async move { Ok(name) }.must_box())
    }
}

impl ghost_actor::GhostHandler<ConEvent> for ConEntityImpl {}

impl ConEventHandler for ConEntityImpl {
    fn handle_user_command(
        &mut self,
        cmd: String,
    ) -> ConEventHandlerResult<()> {
        use UserCommand::*;

        match UserCommand::parse(&cmd) {
            Help => {
                let fut = self
                    .c_send
                    .write_raw(UserCommand::help().as_bytes().to_vec());
                Ok(fut)
            }
            Look => {
                let room = self.room.clone();
                let c_send = self.c_send.clone();
                Ok(async move {
                    let look = room.look().await?;
                    c_send.write_raw(look.into_bytes()).await?;
                    Ok(())
                }
                .must_box())
            }
            Say(s) => {
                let room = self.room.clone();
                let msg = format!("[{}] says: '{}'", self.name, s);
                Ok(async move {
                    room.say(msg).await?;
                    Ok(())
                }
                .must_box())
            }
            Yell(s) => {
                let msg = format!("[{}] yells: '{}'", self.name, s);
                let fut = self.world.yell(msg);
                Ok(fut)
            }
            Name(name) => {
                let fut =
                    self.c_send.prompt_set(format!("{}> ", &name).into_bytes());
                self.name = name;
                Ok(fut)
            }
            RoomName(room_name) => {
                let room = self.room.clone();
                let c_send = self.c_send.clone();
                Ok(async move {
                    room.room_name_set(room_name).await?;
                    let look = room.look().await?;
                    c_send.write_raw(look.into_bytes()).await?;
                    Ok(())
                }
                .must_box())
            }
            RoomExit(dir) => {
                let room = self.room.clone();
                Ok(async move {
                    room.room_exit_toggle(dir).await?;
                    Ok(())
                }
                .must_box())
            }
            Move(dir) => {
                let old_room = self.room.clone();
                let x_s = self.external_sender.clone();
                let world = self.world.clone();
                let new_room_key = dir.translate_room_key(&self.cur_room);
                let c_send = self.c_send.clone();
                Ok(async move {
                    if !old_room.room_has_exit(dir).await? {
                        c_send
                            .write_raw(b"you cannot go that way".to_vec())
                            .await?;
                        return Ok(());
                    }

                    let new_room = world.room_get(new_room_key).await?;

                    let f1 = new_room.entity_hold(x_s.clone());
                    let f2 = old_room.entity_drop(x_s);

                    let _ = futures::future::join(f1, f2).await;

                    let look = new_room.look().await?;
                    c_send.write_raw(look.into_bytes()).await?;
                    Ok(())
                }
                .must_box())
            }
            Quit => {
                let x_s = self.external_sender.clone();
                Ok(async move {
                    x_s.ghost_actor_shutdown_immediate().await?;
                    Ok(())
                }
                .must_box())
            }
            Unknown(s) => {
                let fut = self.c_send.write_raw(s.into_bytes());
                Ok(fut)
            }
        }
    }

    fn handle_destroy(&mut self) -> ConEventHandlerResult<()> {
        use futures::future::TryFutureExt;
        Ok(self
            .external_sender
            .ghost_actor_shutdown_immediate()
            .map_err(|e| e.into())
            .must_box())
    }
}

enum UserCommand {
    Help,
    Look,
    Say(String),
    Yell(String),
    Name(String),
    RoomName(String),
    RoomExit(Direction),
    Move(Direction),
    Quit,
    Unknown(String),
}

fn parse_dir(u: &str) -> Option<Direction> {
    match u.chars().next() {
        Some('n') | Some('N') => Some(Direction::North),
        Some('e') | Some('E') => Some(Direction::East),
        Some('s') | Some('S') => Some(Direction::South),
        Some('w') | Some('W') => Some(Direction::West),
        Some('u') | Some('U') => Some(Direction::Up),
        Some('d') | Some('D') => Some(Direction::Down),
        _ => None,
    }
}

impl UserCommand {
    pub fn help() -> &'static str {
        "\r
GhostActor Mud Example Commands:\r
  help                    - list these commands\r
  look                    - look around the room you are in\r
   say /what to say/      - say something in this room\r
  yell /what to yell/     - yell something everyone can hear\r
  name /your new name/    - rename yourself\r
  room name /room name/   - rename this room\r
  room exit /DIR/         - toggle an exit from this room in direction /DIR/\r
  move /DIR/              - move in direction /DIR/\r
  quit                    - disconnect / leave the mud\r

/DIR/ections:
  north, east, south, west, up, down
"
    }

    pub fn parse(u: &str) -> Self {
        match u.chars().next() {
            Some('h') | Some('H') => UserCommand::Help,
            Some('l') | Some('L') => UserCommand::Look,
            Some('s') | Some('S') => {
                let idx = u.find(char::is_whitespace).unwrap_or(u.len());
                UserCommand::Say(u[idx..].trim().to_string())
            }
            Some('y') | Some('Y') => {
                let idx = u.find(char::is_whitespace).unwrap_or(u.len());
                UserCommand::Yell(u[idx..].trim().to_string())
            }
            Some('n') | Some('N') => {
                let idx = u.find(char::is_whitespace).unwrap_or(u.len());
                UserCommand::Name(u[idx..].trim().to_string())
            }
            Some('r') | Some('R') => {
                let idx = u.find(char::is_whitespace).unwrap_or(u.len());
                let u = u[idx..].trim();
                match u.chars().next() {
                    Some('n') | Some('N') => {
                        let idx =
                            u.find(char::is_whitespace).unwrap_or(u.len());
                        UserCommand::RoomName(u[idx..].trim().to_string())
                    }
                    Some('e') | Some('E') => {
                        let idx =
                            u.find(char::is_whitespace).unwrap_or(u.len());
                        let u = u[idx..].trim();
                        match parse_dir(u) {
                            Some(dir) => UserCommand::RoomExit(dir),
                            None => {
                                let idx = u
                                    .find(char::is_whitespace)
                                    .unwrap_or(u.len());
                                UserCommand::Unknown(format!(
                                    "unknown room exit direction: '{}'",
                                    &u[..idx]
                                ))
                            }
                        }
                    }
                    _ => {
                        let idx =
                            u.find(char::is_whitespace).unwrap_or(u.len());
                        UserCommand::Unknown(format!(
                            "unknown room subcommand: '{}'",
                            &u[..idx]
                        ))
                    }
                }
            }
            Some('m') | Some('M') => {
                let idx = u.find(char::is_whitespace).unwrap_or(u.len());
                let u = u[idx..].trim();
                match parse_dir(u) {
                    Some(dir) => UserCommand::Move(dir),
                    None => {
                        let idx =
                            u.find(char::is_whitespace).unwrap_or(u.len());
                        UserCommand::Unknown(format!(
                            "unknown move direction: '{}'",
                            &u[..idx]
                        ))
                    }
                }
            }
            Some('q') | Some('Q') => UserCommand::Quit,
            _ => {
                let idx = u.find(char::is_whitespace).unwrap_or(u.len());
                UserCommand::Unknown(format!(
                    "unknown command: '{}'",
                    &u[..idx]
                ))
            }
        }
    }
}
