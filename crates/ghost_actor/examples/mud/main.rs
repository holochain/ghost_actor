use ghost_actor::{dependencies::must_future::*, GhostControlSender};
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

    // set up a basic world
    starting_world(world.clone()).await;

    let mut listener =
        tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap();
    println!("telnet 127.0.0.1 {}", listener.local_addr().unwrap().port());

    while let Some(Ok(socket)) = listener.next().await {
        println!("got connection: {}", socket.peer_addr().unwrap());
        tokio::task::spawn(socket_task(world.clone(), socket));
    }
}

async fn socket_task(
    world: ghost_actor::GhostSender<World>,
    socket: tokio::net::TcpStream,
) {
    let (c_send, c_recv) = spawn_con(socket).await;

    let room = world.room_get((0, 0, 0)).await.unwrap();

    let entity = spawn_con_entity(world.clone(), c_send, c_recv).await;

    room.entity_hold(entity).await.unwrap();
}

async fn starting_world(world: ghost_actor::GhostSender<World>) {
    let mut all = Vec::new();

    let r = futures::future::join_all(vec![
        world.room_get((0, 0, 0).into()),  // start
        world.room_get((0, 0, -1).into()), // down one
        world.room_get((0, 1, 0).into()),  // north one
    ])
    .await
    .into_iter()
    .map(|r| r.unwrap())
    .collect::<Vec<_>>();

    // starting room
    all.push(r[0].room_name_set("Welcoming Courtyard".to_string()));
    all.push(r[0].room_exit_toggle(Direction::North));
    all.push(r[0].room_exit_toggle(Direction::Down));

    // down one
    all.push(r[1].room_name_set("A Dank Well".to_string()));
    all.push(r[1].room_exit_toggle(Direction::Up));

    // north one
    all.push(r[2].room_name_set("A Forbidding Desert".to_string()));
    all.push(r[2].room_exit_toggle(Direction::South));

    for i in futures::future::join_all(all).await {
        i.unwrap();
    }
}
