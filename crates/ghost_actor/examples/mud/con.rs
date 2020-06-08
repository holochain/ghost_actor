use crate::*;

ghost_actor::ghost_chan! {
    /// Incoming events from the connection.
    pub chan ConEvent<MudError> {
        fn user_command(cmd: Vec<u8>) -> ();
    }
}

pub type ConEventReceiver = futures::channel::mpsc::Receiver<ConEvent>;

ghost_actor::ghost_actor! {
    /// A connected mud client.
    pub actor Con<MudError> {
        fn write_raw(msg: Vec<u8>) -> ();
    }
}

pub async fn spawn_con(socket: tokio::net::TcpStream) -> (ConSender, ConEventReceiver) {
    let (mut read_half, mut write_half) = socket.into_split();

    // open a channel for the write task
    let (wsend, mut wrecv) = tokio::sync::mpsc::channel::<(
        Vec<u8>,
        tokio::sync::oneshot::Sender<tokio::io::Result<()>>,
    )>(10);

    // spawn the write task
    tokio::task::spawn(async move {
        while let Some((data, resp)) = wrecv.next().await {
            let res = write_half.write_all(&data).await;
            let _ = resp.send(res);
        }
    });

    // open a channel for the read task
    let (mut rsend, rrecv) = futures::channel::mpsc::channel(10);

    // spawn the read task
    tokio::task::spawn(async move {
        while let Ok(c) = read_half.read_u8().await {
            rsend.user_command(vec![c]).await.unwrap();
        }
    });

    // spawn the actor impl
    let (sender, driver) = ConSender::ghost_actor_spawn(Box::new(|_i_s| {
        async move { Ok(ConImpl { write_send: wsend }) }
            .boxed()
            .into()
    }))
    .await
    .unwrap();

    tokio::task::spawn(driver);

    (sender, rrecv)
}

struct ConImpl {
    write_send:
        tokio::sync::mpsc::Sender<(Vec<u8>, tokio::sync::oneshot::Sender<tokio::io::Result<()>>)>,
}

impl ConHandler<(), ()> for ConImpl {
    fn handle_write_raw(&mut self, msg: Vec<u8>) -> ConHandlerResult<()> {
        let mut write_send = self.write_send.clone();
        Ok(async move {
            let (rsend, rrecv) = tokio::sync::oneshot::channel();
            write_send.send((msg, rsend)).await?;
            rrecv.await??;
            Ok(())
        }
        .boxed()
        .into())
    }
}
