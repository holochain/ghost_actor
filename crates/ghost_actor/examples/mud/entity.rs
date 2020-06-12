use crate::*;

ghost_actor::ghost_actor! {
    /// An entity represents a user / npc / or item that can move around.
    pub actor Entity<MudError> {
        fn say(msg: String) -> ();
        fn room_set(room_key: RoomKey) -> ();
    }
}

pub async fn spawn_con_entity(
    world: WorldSender,
    mut c_send: ConSender,
    c_recv: ConEventReceiver,
) -> EntitySender {
    c_send
        .prompt_set(b"ghost_actor_mud> ".to_vec())
        .await
        .unwrap();

    let (sender, driver) = EntitySender::ghost_actor_spawn(|i_s| {
        async move { Ok(ConEntityImpl::new(i_s, world, c_send, c_recv)) }.must_box()
    })
    .await
    .unwrap();

    tokio::task::spawn(driver);

    sender
}

struct ConEntityImpl {
    internal_sender: EntityInternalSender<EntityInner>,
    world: WorldSender,
    cur_room: RoomKey,
    c_send: ConSender,
}

impl ConEntityImpl {
    pub fn new(
        internal_sender: EntityInternalSender<EntityInner>,
        world: WorldSender,
        c_send: ConSender,
        mut c_recv: ConEventReceiver,
    ) -> Self {
        let mut i_s = internal_sender.clone();
        tokio::task::spawn(async move {
            while let Some(evt) = c_recv.next().await {
                if let Err(_) = i_s.ghost_actor_internal().con_recv(evt).await {
                    break;
                }
            }
        });

        Self {
            internal_sender,
            world,
            cur_room: (0, 0, 0),
            c_send,
        }
    }
}

impl EntityHandler<(), EntityInner> for ConEntityImpl {
    fn handle_say(&mut self, _msg: String) -> EntityHandlerResult<()> {
        Ok(async move { Ok(()) }.must_box())
    }

    fn handle_room_set(&mut self, room_key: RoomKey) -> EntityHandlerResult<()> {
        self.cur_room = room_key;
        Ok(async move { Ok(()) }.must_box())
    }

    fn handle_ghost_actor_internal(&mut self, input: EntityInner) -> EntityResult<()> {
        tokio::task::spawn(input.dispatch(self));
        Ok(())
    }
}

ghost_actor::ghost_chan! {
    chan EntityInner<MudError> {
        fn con_recv(evt: ConEvent) -> ();
    }
}

impl EntityInnerHandler for ConEntityImpl {
    fn handle_con_recv(&mut self, evt: ConEvent) -> EntityInnerHandlerResult<()> {
        use futures::future::FutureExt;
        Ok(evt.dispatch(self).map(|_|Ok(())).must_box())
    }
}

impl ConEventHandler for ConEntityImpl {
    fn handle_user_command(&mut self, cmd: String) -> ConEventHandlerResult<()> {
        let mut c_send = self.c_send.clone();
        Ok(async move {
            c_send
                .write_raw(format!("you say: '{}'", cmd).into_bytes())
                .await?;
            Ok(())
        }.must_box())
    }

    fn handle_destroy(&mut self) -> ConEventHandlerResult<()> {
        Ok(async move { Ok(()) }.must_box())
    }
}
