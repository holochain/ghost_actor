use std::sync::Arc;
use std::time::Duration;

use criterion::criterion_group;
use criterion::criterion_main;
use criterion::BenchmarkId;
use criterion::Criterion;

use tokio::runtime::Builder;
use tokio::runtime::Runtime;

use ghost_actor::*;

trait Fruit {
    fn eat(&self, w: u64, r: u64) -> GhostFuture<u64, GhostError>;
    fn look(&self, r: u64) -> GhostFuture<u64, GhostError>;
}

#[derive(Debug, Clone)]
pub struct Banana(GhostActor<u64>);

impl Banana {
    pub fn new() -> Self {
        let (actor, driver) = GhostActor::new(0);
        tokio::task::spawn(driver);
        Self(actor)
    }
}

impl Fruit for Banana {
    fn eat(&self, w: u64, r: u64) -> GhostFuture<u64, GhostError> {
        let actor = self.0.clone();

        resp(async move {
            let count = actor
                .invoke::<_, GhostError, _>(move |count| {
                    if w > 0 {
                        std::thread::sleep(Duration::from_nanos(w));
                    }
                    *count += 1;
                    Ok(*count)
                })
                .await?;
            if r > 0 {
                tokio::time::sleep(Duration::from_nanos(r)).await;
            }

            Ok(count)
        })
    }
    fn look(&self, r: u64) -> GhostFuture<u64, GhostError> {
        let actor = self.0.clone();

        resp(async move {
            let count =
                actor.invoke::<_, GhostError, _>(|count| Ok(*count)).await?;

            if r > 0 {
                tokio::time::sleep(Duration::from_nanos(r)).await;
            }
            Ok(count)
        })
    }
}

#[derive(Debug)]
struct LockActor<T> {
    t: tokio::sync::RwLock<T>,
}

impl<T> LockActor<T> {
    pub fn new(t: T) -> Self {
        Self {
            t: tokio::sync::RwLock::new(t),
        }
    }
    pub async fn invoke_mut<R, F>(&self, invoke: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut g = self.t.write().await;
        let r = invoke(&mut g);
        r
    }
    pub async fn invoke_ref<R, F>(&self, invoke: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let g = self.t.read().await;
        let r = invoke(&g);
        r
    }
}

#[derive(Debug, Clone)]
struct BananaLock(Arc<LockActor<u64>>);

impl BananaLock {
    async fn eat(&self, w: u64, r: u64) -> u64 {
        let n = self
            .0
            .invoke_mut(|n| {
                if w > 0 {
                    std::thread::sleep(Duration::from_nanos(w));
                }
                *n += 1;
                *n
            })
            .await;
        if r > 0 {
            tokio::time::sleep(Duration::from_nanos(r)).await;
        }
        n
    }
    async fn look(&self, r: u64) -> u64 {
        let n = self.0.invoke_ref(|n| *n).await;
        if r > 0 {
            tokio::time::sleep(Duration::from_nanos(r)).await;
        }
        n
    }
}

criterion_group!(benches, simple_bench, concurrent_bench, load_bench);

criterion_main!(benches);

fn simple_bench(bench: &mut Criterion) {
    let _g = observability::test_run().ok();

    let runtime = rt();

    let actor = runtime.block_on(setup());
    let lock = runtime.block_on(banana_lock());

    let mut group = bench.benchmark_group("simple_bench");
    group.bench_function(BenchmarkId::new("banana", "eat"), |b| {
        b.iter(|| {
            runtime.block_on(async { actor.eat(0, 0).await.unwrap() });
        });
    });
    group.bench_function(BenchmarkId::new("lock", "eat"), |b| {
        b.iter(|| {
            runtime.block_on(async { lock.eat(0, 0).await });
        });
    });
    group.bench_function(BenchmarkId::new("banana", "look"), |b| {
        b.iter(|| {
            runtime.block_on(async {
                actor.look(0).await.unwrap();
            });
        });
    });
    group.bench_function(BenchmarkId::new("lock", "look"), |b| {
        b.iter(|| {
            runtime.block_on(async {
                lock.look(0).await;
            });
        });
    });
    runtime.block_on(async move { drop(actor) });
}

fn concurrent_bench(bench: &mut Criterion) {
    let _g = observability::test_run().ok();
    let (num_con, write_len, read_len) = std::env::var_os("GHOST_BENCH").map(|s| {
        let s = s.to_string_lossy();
        s.split(',').map(|s| s.parse::<usize>().expect("GHOST_BENCH must be numbers. <number_of_concurrent> <read_len> <write_len>")).collect::<Vec<_>>()
    }).map(|s|{
        let mut s = s.into_iter();
        (s.next().unwrap_or(1),
        s.next().unwrap_or(100) as u64,
        s.next().unwrap_or(100) as u64)
    }).unwrap_or((1, 100, 100));

    let runtime = rt();

    let actor = runtime.block_on(setup());
    let lock = runtime.block_on(banana_lock());

    let mut group = bench.benchmark_group("concurrent_bench");

    group.bench_function(
        BenchmarkId::new(
            "banana_eat",
            format!("eat_{}_w_{}_r_{}", num_con, write_len, read_len),
        ),
        |b| {
            b.iter(|| {
                runtime.block_on(con_banana_eat(
                    &actor, num_con, read_len, write_len,
                ));
            });
        },
    );
    group.bench_function(
        BenchmarkId::new(
            "lock_eat",
            format!("eat_{}_w_{}_r_{}", num_con, write_len, read_len),
        ),
        |b| {
            b.iter(|| {
                runtime.block_on(con_lock_eat(
                    &lock, num_con, read_len, write_len,
                ));
            });
        },
    );
    group.bench_function(
        BenchmarkId::new(
            "banana_look",
            format!("look_{}_w_{}_r_{}", num_con, write_len, read_len),
        ),
        |b| {
            b.iter(|| {
                runtime.block_on(con_banana_look(&actor, num_con, read_len));
            });
        },
    );
    group.bench_function(
        BenchmarkId::new(
            "lock_look",
            format!("look_{}_w_{}_r_{}", num_con, write_len, read_len),
        ),
        |b| {
            b.iter(|| {
                runtime.block_on(con_lock_look(&lock, num_con, read_len));
            });
        },
    );
    runtime.block_on(async move { drop(actor) });
}

fn load_bench(bench: &mut Criterion) {
    let _g = observability::test_run().ok();
    let (num_con, write_len, read_len) = std::env::var_os("GHOST_BENCH").map(|s| {
        let s = s.to_string_lossy();
        s.split(',').map(|s| s.parse::<usize>().expect("GHOST_BENCH must be numbers. <number_of_concurrent> <read_len> <write_len>")).collect::<Vec<_>>()
    }).map(|s|{
        let mut s = s.into_iter();
        (s.next().unwrap_or(1),
        s.next().unwrap_or(100) as u64,
        s.next().unwrap_or(100) as u64)
    }).unwrap_or((1, 100, 100));

    let mut group = bench.benchmark_group("load_bench");
    let runtime = rt();

    let actor = runtime.block_on(setup());
    runtime.spawn(load_banana_eat(actor.clone(), num_con, read_len, write_len));
    group.bench_function(
        BenchmarkId::new(
            "banana_eat",
            format!("load_{}_w_{}_r_{}", num_con, write_len, read_len),
        ),
        |b| {
            b.iter(|| {
                runtime.block_on(async {
                    actor.eat(write_len, read_len).await.unwrap()
                });
            });
        },
    );
    runtime.block_on(async move { drop(actor) });
    runtime.shutdown_background();

    let runtime = rt();
    let lock = runtime.block_on(banana_lock());
    runtime.spawn(load_lock_eat(lock.clone(), num_con, read_len, write_len));
    group.bench_function(
        BenchmarkId::new(
            "lock_eat",
            format!("load_{}_w_{}_r_{}", num_con, write_len, read_len),
        ),
        |b| {
            b.iter(|| {
                runtime.block_on(async { lock.eat(write_len, read_len).await });
            });
        },
    );
    runtime.shutdown_background();
}

async fn con_banana_eat(
    actor: &Banana,
    num_con: usize,
    read_len: u64,
    write_len: u64,
) {
    let mut jhs = Vec::new();
    for _ in 0..num_con {
        let actor = actor.clone();
        let jh = tokio::spawn(async move {
            actor.eat(write_len, read_len).await.unwrap();
        });
        jhs.push(jh);
    }
    for jh in jhs {
        jh.await.unwrap();
    }
}

async fn con_lock_eat(
    lock: &BananaLock,
    num_con: usize,
    read_len: u64,
    write_len: u64,
) {
    let mut jhs = Vec::new();
    for _ in 0..num_con {
        let lock = lock.clone();
        let jh = tokio::spawn(async move {
            lock.eat(write_len, read_len).await;
        });
        jhs.push(jh);
    }
    for jh in jhs {
        jh.await.unwrap();
    }
}
async fn con_banana_look(actor: &Banana, num_con: usize, read_len: u64) {
    let mut jhs = Vec::new();
    for _ in 0..num_con {
        let actor = actor.clone();
        let jh = tokio::spawn(async move {
            actor.look(read_len).await.unwrap();
        });
        jhs.push(jh);
    }
    for jh in jhs {
        jh.await.unwrap();
    }
}
async fn con_lock_look(lock: &BananaLock, num_con: usize, read_len: u64) {
    let mut jhs = Vec::new();
    for _ in 0..num_con {
        let lock = lock.clone();
        let jh = tokio::spawn(async move {
            lock.look(read_len).await;
        });
        jhs.push(jh);
    }
    for jh in jhs {
        jh.await.unwrap();
    }
}
async fn load_banana_eat(
    actor: Banana,
    num_con: usize,
    read_len: u64,
    write_len: u64,
) {
    let mut jhs = Vec::new();
    for _ in 0..num_con {
        let actor = actor.clone();
        let jh = tokio::spawn(async move {
            loop {
                if let Err(e) = actor.eat(write_len, read_len).await {
                    tracing::warn!(ghost_load_failed = ?e);
                }
            }
        });
        jhs.push(jh);
    }
    // for jh in jhs {
    //     jh.await.unwrap();
    // }
}

async fn load_lock_eat(
    lock: BananaLock,
    num_con: usize,
    read_len: u64,
    write_len: u64,
) {
    let mut jhs = Vec::new();
    for _ in 0..num_con {
        let lock = lock.clone();
        let jh = tokio::spawn(async move {
            loop {
                lock.eat(write_len, read_len).await;
            }
        });
        jhs.push(jh);
    }
    // for jh in jhs {
    //     jh.await.unwrap();
    // }
}
async fn setup() -> Banana {
    Banana::new()
}

async fn banana_lock() -> BananaLock {
    BananaLock(Arc::new(LockActor::new(0)))
}

pub fn rt() -> Runtime {
    Builder::new_multi_thread().enable_all().build().unwrap()
}
