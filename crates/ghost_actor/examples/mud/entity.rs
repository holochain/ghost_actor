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

    /*
    let i_s = builder
        .channel_factory()
        .create_channel::<EntityInner>()
        .await
        .unwrap();
    */

    builder
        .channel_factory()
        .attach_receiver(c_recv)
        .await
        .unwrap();

    tokio::task::spawn(builder.spawn(ConEntityImpl::new(
        sender.clone(),
        //i_s,
        world,
        c_send,
        //c_recv,
    )));

    sender
}

struct ConEntityImpl {
    external_sender: ghost_actor::GhostSender<Entity>,
    //internal_sender: ghost_actor::GhostSender<EntityInner>,
    world: ghost_actor::GhostSender<World>,
    cur_room: RoomKey,
    c_send: ghost_actor::GhostSender<Con>,
}

impl ConEntityImpl {
    pub fn new(
        external_sender: ghost_actor::GhostSender<Entity>,
        //internal_sender: ghost_actor::GhostSender<EntityInner>,
        world: ghost_actor::GhostSender<World>,
        c_send: ghost_actor::GhostSender<Con>,
        //mut c_recv: ConEventReceiver,
    ) -> Self {
        /*
        let mut i_s = internal_sender.clone();
        tokio::task::spawn(async move {
            while let Some(evt) = c_recv.next().await {
                if let Err(_) = i_s.ghost_actor_internal().con_recv(evt).await {
                    break;
                }
            }
        });
        */

        Self {
            external_sender,
            //internal_sender,
            world,
            cur_room: (0, 0, 0),
            c_send,
        }
    }
}

impl ghost_actor::GhostControlHandler for ConEntityImpl {}

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

    /*
    fn handle_ghost_actor_internal(
        &mut self,
        input: EntityInner,
    ) -> EntityResult<()> {
        tokio::task::spawn(input.dispatch(self));
        Ok(())
    }
    */
}

/*
ghost_actor::ghost_chan! {
    chan EntityInner<MudError> {
        fn stub() -> ();
        //fn con_recv(evt: ConEvent) -> ();
    }
}

impl ghost_actor::GhostHandler<EntityInner> for ConEntityImpl {}

impl EntityInnerHandler for ConEntityImpl {
    fn handle_stub(&mut self) -> EntityInnerHandlerResult<()> {
        Ok(async move { Ok(()) }.must_box())
    }
    /*
    fn handle_con_recv(
        &mut self,
        evt: ConEvent,
    ) -> EntityInnerHandlerResult<()> {
        use futures::future::FutureExt;
        Ok(evt.dispatch(self).map(|_| Ok(())).must_box())
    }
    */
}
*/

impl ghost_actor::GhostHandler<ConEvent> for ConEntityImpl {}

impl ConEventHandler for ConEntityImpl {
    fn handle_user_command(
        &mut self,
        cmd: String,
    ) -> ConEventHandlerResult<()> {
        let world = self.world.clone();
        let room_key = self.cur_room.clone();
        let c_send = self.c_send.clone();
        Ok(async move {
            match UserCommand::parse(&cmd) {
                UserCommand::Say(s) => {
                    let room = world.room_get(room_key).await?;
                    room.say(format!("[user] says: '{}'", s)).await?;
                }
                UserCommand::Unknown(s) => {
                    c_send.write_raw(s.into_bytes()).await?;
                }
            }

            Ok(())
        }
        .must_box())
    }

    fn handle_destroy(&mut self) -> ConEventHandlerResult<()> {
        let x_s = self.external_sender.clone();
        let world = self.world.clone();
        let room_key = self.cur_room.clone();
        Ok(async move {
            let room = world.room_get(room_key).await?;
            room.entity_drop(x_s).await?;
            Ok(())
        }
        .must_box())
    }
}

enum UserCommand {
    Say(String),
    Unknown(String),
}

impl UserCommand {
    pub fn parse(u: &str) -> Self {
        match u.chars().next() {
            Some('s') | Some('S') => {
                let idx = u.find(char::is_whitespace).unwrap_or(u.len());
                UserCommand::Say(u[idx..].trim().to_string())
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
