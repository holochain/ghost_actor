use ghost_actor::dependencies::must_future::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    stream::StreamExt,
};

mod error;
use error::*;

mod con;
use con::*;

mod world;
use world::*;

mod room;
use room::*;

#[tokio::main(threaded_scheduler)]
async fn main() {
    tokio::task::spawn(listener_task());

    let _world = spawn_world().await;

    loop {
        tokio::time::delay_for(std::time::Duration::from_millis(5000)).await;
    }
}

async fn listener_task() {
    let mut listener =
        tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    println!("telnet 127.0.0.1 {}", listener.local_addr().unwrap().port());

    while let Some(Ok(socket)) = listener.next().await {
        println!("got connection: {}", socket.peer_addr().unwrap());
        tokio::task::spawn(socket_task(socket));
    }
}

async fn socket_task(socket: tokio::net::TcpStream) {
    let (mut csend, mut crecv) = spawn_con(socket).await;
    csend
        .prompt_set(b"ghost_actor_mud> ".to_vec())
        .await
        .unwrap();

    while let Some(msg) = crecv.next().await {
        match msg {
            ConEvent::UserCommand { respond, cmd, .. } => {
                respond.respond(Ok(()));
                csend
                    .write_raw(format!("you say: '{}'", cmd).into_bytes())
                    .await
                    .unwrap();
            }
        }
    }
}
