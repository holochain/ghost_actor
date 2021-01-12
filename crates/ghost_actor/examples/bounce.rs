use ghost_actor::*;

/// Makes return types out of `invoke()` closures easier.
pub type Result<T> = std::result::Result<T, GhostError>;

/// Generic entity that can exist in a "World".
pub trait Entity: 'static + Send {
    /// Facilitates cloning "BoxEntity" instances.
    fn box_clone(&self) -> Box<dyn Entity>;

    /// Get the position (and character) of this entity in the World.
    fn pos(&self) -> GhostFuture<(char, u8, u8), GhostError>;
}

/// Type erased entity.
pub type BoxEntity = Box<dyn Entity>;

impl Clone for BoxEntity {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

/// inner state data for entity with no gravity
struct NoGravityInner {
    x: i8,
    vx: i8,
    y: i8,
    vy: i8,
}

/// An entity that moves as if there is no gravity
#[derive(Clone)]
struct NoGravity(GhostActor<NoGravityInner>);

impl NoGravity {
    /// Create a new entity with starting position / velocity.
    pub fn new(x: i8, vx: i8, y: i8, vy: i8) -> BoxEntity {
        let (actor, driver) = GhostActor::new(NoGravityInner { x, vx, y, vy });
        tokio::task::spawn(driver);
        let out = Self(actor.clone());

        // spawn our task for handling this entity's movement
        tokio::task::spawn(async move {
            while actor.is_active() {
                actor
                    .invoke(move |inner| {
                        inner.x += inner.vx;
                        inner.y += inner.vy;
                        if inner.x >= 16 {
                            inner.vx = -1;
                        }
                        if inner.y >= 8 {
                            inner.vy = -1;
                        }
                        if inner.x <= 1 {
                            inner.vx = 1;
                        }
                        if inner.y <= 1 {
                            inner.vy = 1;
                        }
                        Result::Ok(())
                    })
                    .await?;

                tokio::time::delay_for(std::time::Duration::from_millis(50))
                    .await;
            }
            Result::Ok(())
        });
        Box::new(out)
    }
}

impl Entity for NoGravity {
    fn box_clone(&self) -> Box<dyn Entity> {
        Box::new(self.clone())
    }

    fn pos(&self) -> GhostFuture<(char, u8, u8), GhostError> {
        let fut = self
            .0
            .invoke(|inner| Result::Ok(('O', inner.x as u8, inner.y as u8)));

        resp(async move { fut.await })
    }
}

/// inner state data for entity that experiences gravity
struct GravityInner {
    x: f32,
    vx: f32,
    y: f32,
    vy: f32,
}

/// An entity that moves as if there is gravity (but no friction / air)
#[derive(Clone)]
struct Gravity(GhostActor<GravityInner>);

impl Gravity {
    /// Create a new entity with starting position / velocity.
    pub fn new(x: f32, vx: f32, y: f32, vy: f32) -> BoxEntity {
        const G: f32 = 0.1;
        let (actor, driver) = GhostActor::new(GravityInner { x, vx, y, vy });
        tokio::task::spawn(driver);
        let out = Self(actor.clone());
        tokio::task::spawn(async move {
            while actor.is_active() {
                actor
                    .invoke(move |inner| {
                        inner.vy += G;
                        inner.x += inner.vx;
                        inner.y += inner.vy;
                        if inner.x >= 16.0 {
                            inner.vx = -inner.vx;
                            inner.x -= inner.x - 16.0;
                        }
                        if inner.y >= 8.0 {
                            inner.vy = -inner.vy;
                            inner.y -= inner.y - 8.0;
                            if inner.vy.abs() < 0.2 {
                                inner.vy = -1.2;
                            }
                        }
                        if inner.x <= 1.0 {
                            inner.vx = -inner.vx;
                            inner.x += 1.0 - inner.x;
                        }
                        if inner.y <= 1.0 {
                            inner.vy = -inner.vy;
                            inner.y += 1.0 - inner.y;
                        }
                        Result::Ok(())
                    })
                    .await?;

                // target ~ 50 fps
                tokio::time::delay_for(std::time::Duration::from_millis(20))
                    .await;
            }
            Result::Ok(())
        });
        Box::new(out)
    }
}

impl Entity for Gravity {
    fn box_clone(&self) -> Box<dyn Entity> {
        Box::new(self.clone())
    }

    fn pos(&self) -> GhostFuture<(char, u8, u8), GhostError> {
        let fut = self.0.invoke(|inner| {
            Result::Ok(('#', inner.x.round() as u8, inner.y.round() as u8))
        });

        resp(async move { fut.await })
    }
}

/// World inner state data is just a list of entities
type WorldInner = Vec<BoxEntity>;

/// A world contains entities.
pub struct World(GhostActor<WorldInner>);

impl World {
    /// initialize a new World instance.
    pub fn new() -> Self {
        let (actor, driver) = GhostActor::new(Vec::new());
        tokio::task::spawn(driver);
        Self(actor)
    }

    /// is this world still active?
    pub fn is_active(&self) -> bool {
        self.0.is_active()
    }

    /// add an entity to this world
    pub async fn add_entity(&self, entity: BoxEntity) -> Result<()> {
        self.0
            .invoke(move |inner| {
                inner.push(entity);
                Result::Ok(())
            })
            .await?;
        Ok(())
    }

    /// get the positions + characters of all entities in this World.
    pub async fn draw(&self) -> Result<Vec<(char, u8, u8)>> {
        let entities: Vec<BoxEntity> = self
            .0
            .invoke(|inner| {
                Result::Ok(inner.iter().map(|x| x.clone()).collect())
            })
            .await?;

        let mut out = Vec::new();

        for pos in
            futures::future::join_all(entities.into_iter().map(|e| e.pos()))
                .await
        {
            out.push(pos?);
        }

        Ok(out)
    }
}

#[tokio::main]
pub async fn main() -> Result<()> {
    // draw the board
    print!("\x1b[2J\x1b[1;1H");
    print!("+----------------+\n");
    for _ in 0..8 {
        print!("|                |\n");
    }
    print!("+----------------+\n");

    // construct the actors
    let world = World::new();
    world.add_entity(NoGravity::new(0, 1, 0, 1)).await?;
    world.add_entity(Gravity::new(15.0, -0.2, 0.0, 0.0)).await?;

    // render loop
    let mut prev_points: Vec<(u8, u8)> = Vec::new();
    while world.is_active() {
        let mut draw = String::new();

        for (x, y) in prev_points.drain(..) {
            // erase previous points
            draw.push_str(&format!("\x1b[{};{}H ", y, x));
        }

        for (c, mut x, mut y) in world.draw().await? {
            // account for border
            x = x + 1;
            y = y + 1;
            // register to be erased next loop
            prev_points.push((x, y));
            // draw character at position
            draw.push_str(&format!("\x1b[{};{}H{}", y, x, c));
        }

        // set cursor right after board so it doesn't interfere
        draw.push_str("\x1b[11;1H");

        // draw and flush all at once to mitigate flashing
        print!("{}", draw);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        // target ~ 50 fps
        tokio::time::delay_for(std::time::Duration::from_millis(20)).await;
    }

    Ok(())
}
