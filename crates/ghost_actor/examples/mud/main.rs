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

mod entity;
use entity::*;

#[tokio::main(threaded_scheduler)]
async fn main() {
    let world = spawn_world().await;

    let mut listener =
        tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    println!("telnet 127.0.0.1 {}", listener.local_addr().unwrap().port());

    while let Some(Ok(socket)) = listener.next().await {
        println!("got connection: {}", socket.peer_addr().unwrap());
        tokio::task::spawn(socket_task(world.clone(), socket));
    }
}

async fn socket_task(mut world: WorldSender, socket: tokio::net::TcpStream) {
    let (c_send, c_recv) = spawn_con(socket).await;

    let mut room = world.room_get((0, 0, 0)).await.unwrap();

    let entity = spawn_con_entity(world.clone(), c_send, c_recv).await;

    room.entity_hold(entity).await.unwrap();
}
