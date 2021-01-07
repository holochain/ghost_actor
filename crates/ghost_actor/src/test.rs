#[tokio::test]
async fn full_actor_workflow_test() {
    use crate::*;

    trait Fruit {
        fn eat(&self) -> InvokeResult<String, GhostError>;
    }

    pub struct Banana(GhostActor<u32>);

    impl Banana {
        pub fn new() -> Self {
            let (actor, driver) = GhostActor::new(0);
            tokio::task::spawn(driver);
            Self(actor)
        }
    }

    impl Fruit for Banana {
        fn eat(&self) -> InvokeResult<String, GhostError> {
            let actor = self.0.clone();

            resp(async move {
                let count = actor
                    .invoke(|count: &mut u32| {
                        *count += 1;
                        Ok(*count)
                    })
                    .await?;

                Ok(format!("ate {} bananas", count))
            })
        }
    }

    let banana = Banana::new();
    assert_eq!("ate 1 bananas", &banana.eat().await.unwrap(),);

    let fruit: Box<dyn Fruit> = Box::new(banana);
    assert_eq!("ate 2 bananas", &fruit.eat().await.unwrap(),);
}
