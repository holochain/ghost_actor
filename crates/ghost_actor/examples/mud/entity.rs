use crate::*;

ghost_actor::ghost_chan! {
    /// An entity represents a user / npc / or item that can move around.
    pub chan Entity<MudError> {
        fn say(msg: String) -> ();
        fn room_set(room_key: RoomKey) -> ();
    }
}

pub async fn spawn_con_entity(
    world: ghost_actor::GhostSender<World>,
    c_send: ghost_actor::GhostSender<Con>,
    c_recv: ConEventReceiver,
) -> ghost_actor::GhostSender<Entity> {
    c_send
        .prompt_set(b"ghost_actor_mud> ".to_vec())
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

    tokio::task::spawn(builder.spawn(ConEntityImpl::new(
        sender.clone(),
        world,
        c_send,
    )));

    sender
}

struct ConEntityImpl {
    external_sender: ghost_actor::GhostSender<Entity>,
    world: ghost_actor::GhostSender<World>,
    cur_room: RoomKey,
    c_send: ghost_actor::GhostSender<Con>,
}

impl ConEntityImpl {
    pub fn new(
        external_sender: ghost_actor::GhostSender<Entity>,
        world: ghost_actor::GhostSender<World>,
        c_send: ghost_actor::GhostSender<Con>,
    ) -> Self {
        Self {
            external_sender,
            world,
            cur_room: (0, 0, 0),
            c_send,
        }
    }
}

impl ghost_actor::GhostControlHandler for ConEntityImpl {
    fn handle_ghost_actor_shutdown(self) -> must_future::MustBoxFuture<'static, ()> {
        let ConEntityImpl {
            external_sender,
            world,
            cur_room,
            c_send,
        } = self;
        must_future::MustBoxFuture::new(async move {
            let room = world.room_get(cur_room).await.unwrap();
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
    ) -> EntityHandlerResult<()> {
        self.cur_room = room_key;
        Ok(async move { Ok(()) }.must_box())
    }
}

impl ghost_actor::GhostHandler<ConEvent> for ConEntityImpl {}

impl ConEventHandler for ConEntityImpl {
    fn handle_user_command(
        &mut self,
        cmd: String,
    ) -> ConEventHandlerResult<()> {
        let x_s = self.external_sender.clone();
        let world = self.world.clone();
        let room_key = self.cur_room.clone();
        let c_send = self.c_send.clone();
        Ok(async move {
            use UserCommand::*;
            match UserCommand::parse(&cmd) {
                Help => {
                    c_send.write_raw(UserCommand::help().as_bytes().to_vec()).await?;
                }
                Look => {
                    c_send.write_raw(b"you look around the room".to_vec()).await?;
                }
                Say(s) => {
                    let room = world.room_get(room_key).await?;
                    room.say(format!("[user] says: '{}'", s)).await?;
                }
                Yell(s) => {
                    //world.yell(format!("[user] yells: '{}'", s)).await?;
                }
                Quit => {
                    x_s.ghost_actor_shutdown_immediate().await?;
                }
                Unknown(s) => {
                    c_send.write_raw(s.into_bytes()).await?;
                }
            }

            Ok(())
        }
        .must_box())
    }

    fn handle_destroy(&mut self) -> ConEventHandlerResult<()> {
        use futures::future::TryFutureExt;
        Ok(self
           .external_sender
           .ghost_actor_shutdown_immediate()
           .map_err(|e|e.into())
           .must_box()
        )
    }
}

enum UserCommand {
    Help,
    Look,
    Say(String),
    Yell(String),
    Quit,
    Unknown(String),
}

impl UserCommand {
    pub fn help() -> &'static str {
"GhostActor Mud Example Commands:\r
  help                    - list these commands\r
  look                    - look around the room you are in\r
  say                     - say something out in this room\r
  yell                    - yell something everyone can hear\r
  quit                    - leave the mud\r
"
}

    pub fn parse(u: &str) -> Self {
        match u.chars().next() {
            Some('h') | Some('H') => {
                UserCommand::Help
            }
            Some('l') | Some('L') => {
                UserCommand::Look
            }
            Some('s') | Some('S') => {
                let idx = u.find(char::is_whitespace).unwrap_or(u.len());
                UserCommand::Say(u[idx..].trim().to_string())
            }
            Some('y') | Some('Y') => {
                let idx = u.find(char::is_whitespace).unwrap_or(u.len());
                UserCommand::Yell(u[idx..].trim().to_string())
            }
            Some('q') | Some('Q') => {
                UserCommand::Quit
            }
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
