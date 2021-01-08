<a href="https://github.com/holochain/ghost_actor/blob/master/LICENSE-APACHE">![Crates.io](https://img.shields.io/crates/l/ghost_actor)</a>
<a href="https://crates.io/crates/ghost_actor">![Crates.io](https://img.shields.io/crates/v/ghost_actor)</a>

# ghost_actor

GhostActor makes it simple, ergonomic, and idiomatic to implement
async / concurrent code using an Actor model.

GhostActor uses only safe code, and is futures executor agnostic--use
tokio, futures, async-std, whatever you want.

## What does it do?

The GhostActor struct is a `'static + Send + Sync` cheaply clone-able
handle for managing rapid, efficient, sequential, mutable access to
internal state data.

##### Basic Example

```rust
// set our initial state
let (a, driver) = GhostActor::new(42_u32);

// spawn the driver--using tokio here as an example
tokio::task::spawn(driver);

// invoke some logic on the internal state (just reading here)
let result: Result<u32, GhostError> = a.invoke(|a| Ok(*a)).await;

// assert the result
assert_eq!(42, result.unwrap());
```

## Chat Room Actor Example

```rust
type MessageList = Vec<String>;

struct ChatState {
    rooms: HashMap<String, MessageList>,
}

impl ChatState {
    fn room(&mut self, room: String) -> &mut MessageList {
        self
            .rooms
            .entry(room)
            .or_insert_with(|| Vec::new())
    }

    fn post(&mut self, room: String, message: String) {
        self.room(room).push(message);
    }

    fn read(&mut self, room: String) -> MessageList {
        self.room(room).clone()
    }
}

#[derive(Clone)]
pub struct ChatServer(GhostActor<ChatState>);

impl ChatServer {
    pub async fn post(&self, room: &str, message: &str) {
        let room = room.to_string();
        let message = message.to_string();
        self.0.invoke(move |state| {
            state.post(room, message);
            Result::<(), GhostError>::Ok(())
        }).await.unwrap();
    }

    pub async fn read(&self, room: &str) -> Vec<String> {
        let room = room.to_string();
        self.0.invoke(move |state| {
            let result = state.read(room);
            Result::<Vec<String>, GhostError>::Ok(result)
        }).await.unwrap()
    }
}

#[tokio::main]
async fn main() {
    let state = ChatState { rooms: HashMap::new() };

    let (actor, driver) = GhostActor::new(state);
    tokio::task::spawn(driver);

    let server1 = ChatServer(actor);
    let server2 = server1.clone();

    futures::future::join_all(vec![
        server1.post("fruit", "banana"),
        server2.post("fruit", "apple"),
    ]).await;

    let mut res = server1.read("fruit").await;
    res.sort();
    assert_eq!(
        vec!["apple".to_string(), "banana".to_string()],
        res,
    );
}
```
