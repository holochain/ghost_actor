use crate::*;

ghost_actor::ghost_chan! {
    /// Incoming events from the connection.
    pub chan ConEvent<MudError> {
        fn user_command(cmd: String) -> ();
        fn destroy() -> ();
    }
}

pub type ConEventReceiver = futures::channel::mpsc::Receiver<ConEvent>;

ghost_actor::ghost_chan! {
    /// A connected mud client.
    pub chan Con<MudError> {
        /// set the prompt the user will see
        fn prompt_set(p: Vec<u8>) -> ();
        /// write raw data to the terminal (prompt will be cleared first)
        fn write_raw(msg: Vec<u8>) -> ();
    }
}

pub async fn spawn_con(
    socket: tokio::net::TcpStream,
) -> (ghost_actor::GhostSender<Con>, ConEventReceiver) {
    // open a channel for the read task
    let (mut rsend, rrecv) = futures::channel::mpsc::channel(10);

    let (mut read_half, mut write_half) = socket.into_split();

    // don't echo
    write_half.write_all(&[0xff, 0xfb, 0x01]).await.unwrap();
    // don't buffer until return
    write_half.write_all(&[0xff, 0xfb, 0x03]).await.unwrap();

    let builder = ghost_actor::actor_builder::GhostActorBuilder::new();

    let sender = builder
        .channel_factory()
        .create_channel::<Con>()
        .await
        .unwrap();

    // spawn the write task
    let write_sender = spawn_write_task(write_half);

    let i_s = sender.clone();

    // spawn the read task
    let write_sender_clone = write_sender.clone();
    tokio::task::spawn(async move {
        let mut cmd = Vec::with_capacity(20);
        let mut wait_command = 0;
        while let Ok(c) = read_half.read_u8().await {
            if wait_command > 0 {
                // mid IAC command - ignore the rest
                wait_command -= 1;
                continue;
            }
            match c {
                255 => {
                    // IAC command - ignore the rest
                    wait_command = 2;
                }
                27 | 3 | 4 => {
                    let _ = ConEventSender::destroy(&mut rsend).await;
                    let _ = write_sender_clone.destroy().await;
                    i_s.ghost_actor_shutdown_immediate().await.unwrap();
                    return;
                }
                8 | 127 => {
                    cmd.pop();
                    write_sender_clone.buffer_set(cmd.clone()).await.unwrap();
                }
                10 | 13 => {
                    write_sender_clone.buffer_add_char(10).await.unwrap();
                    write_sender_clone.buffer_add_char(13).await.unwrap();
                    write_sender_clone
                        .buffer_set(Vec::with_capacity(0))
                        .await
                        .unwrap();
                    if let Ok(s) = std::str::from_utf8(&cmd) {
                        let s = s.trim();
                        if s.len() > 0 {
                            rsend.user_command(s.to_string()).await.unwrap();
                        }
                    }
                    cmd.clear();
                }
                _ => {
                    if cmd.len() < 60 && c >= 0x20 && c <= 0x7e {
                        cmd.push(c);
                        write_sender_clone.buffer_add_char(c).await.unwrap();
                    }
                }
            }
        }
    });

    // spawn the actor impl
    tokio::task::spawn(builder.spawn(ConImpl { write_sender }));

    (sender, rrecv)
}

ghost_actor::ghost_chan! {
    /// Incoming events from the connection.
    chan WriteControl<MudError> {
        fn prompt_set(p: Vec<u8>) -> ();
        fn buffer_add_char(char_: u8) -> ();
        fn buffer_set(buffer: Vec<u8>) -> ();
        fn line_write(line: Vec<u8>) -> ();
        fn destroy() -> ();
    }
}

type WriteControlSend = futures::channel::mpsc::Sender<WriteControl>;

const ERASE_LINE: &'static [u8] = &[
    13, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
    32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
    32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
    32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
    32, 32, 32, 32, 13,
];

fn spawn_write_task(
    mut write_half: tokio::net::tcp::OwnedWriteHalf,
) -> WriteControlSend {
    let (wc_send, mut wc_recv) = futures::channel::mpsc::channel(10);

    tokio::task::spawn(async move {
        let mut prompt: Vec<u8> = Vec::new();
        let mut line_buffer: Vec<u8> = Vec::new();
        while let Some(cmd) = wc_recv.next().await {
            match cmd {
                WriteControl::PromptSet { respond, p, .. } => {
                    // set our prompt
                    prompt = p;
                    // move to start of line
                    write_half.write_all(ERASE_LINE).await.unwrap();
                    // write our current prompt/line_buffer
                    write_half.write_all(&prompt).await.unwrap();
                    write_half.write_all(&line_buffer).await.unwrap();
                    // let our caller know we're done
                    respond.respond(Ok(async move { Ok(()) }.must_box()));
                }
                WriteControl::BufferAddChar { respond, char_, .. } => {
                    // append the char to the line_buffer
                    line_buffer.push(char_);
                    // output the char to the client
                    write_half.write_all(&[char_]).await.unwrap();
                    // let our caller know we're done
                    respond.respond(Ok(async move { Ok(()) }.must_box()));
                }
                WriteControl::BufferSet {
                    respond, buffer, ..
                } => {
                    // set our new prompt/line_buffer
                    line_buffer = buffer;
                    // move to start of line
                    write_half.write_all(ERASE_LINE).await.unwrap();
                    // write our current prompt/line_buffer
                    write_half.write_all(&prompt).await.unwrap();
                    write_half.write_all(&line_buffer).await.unwrap();
                    // let our caller know we're done
                    respond.respond(Ok(async move { Ok(()) }.must_box()));
                }
                WriteControl::LineWrite { respond, line, .. } => {
                    // move to start of line
                    write_half.write_all(ERASE_LINE).await.unwrap();
                    // write the output
                    write_half.write_all(&line).await.unwrap();
                    // move to beginnig of next line
                    write_half.write_all(&[10, 13]).await.unwrap();
                    // write our current prompt/line_buffer
                    write_half.write_all(&prompt).await.unwrap();
                    write_half.write_all(&line_buffer).await.unwrap();
                    // let our caller know we're done
                    respond.respond(Ok(async move { Ok(()) }.must_box()));
                }
                WriteControl::Destroy { respond, .. } => {
                    respond.respond(Ok(async move { Ok(()) }.must_box()));
                    return;
                }
            }
        }
    });

    wc_send
}

struct ConImpl {
    write_sender: WriteControlSend,
}

impl ghost_actor::GhostControlHandler for ConImpl {
    fn handle_ghost_actor_shutdown(
        self,
    ) -> must_future::MustBoxFuture<'static, ()> {
        let ConImpl { write_sender } = self;
        must_future::MustBoxFuture::new(async move {
            let _ = write_sender.destroy().await;
        })
    }
}

impl ghost_actor::GhostHandler<Con> for ConImpl {}

impl ConHandler for ConImpl {
    fn handle_prompt_set(&mut self, p: Vec<u8>) -> ConHandlerResult<()> {
        Ok(self.write_sender.prompt_set(p))
    }

    fn handle_write_raw(&mut self, msg: Vec<u8>) -> ConHandlerResult<()> {
        Ok(self.write_sender.line_write(msg))
    }
}
