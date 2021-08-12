use ghost_actor::*;

/// Custom example error type.
#[derive(Debug, thiserror::Error)]
pub enum MyError {
    /// custom errors must support `From<GhostError>`
    #[error(transparent)]
    GhostError(#[from] crate::GhostError),
}

crate::ghost_chan! {
    /// Test multi-line doc on actor type.
    ///
    /// # Example
    ///
    /// ```
    /// // we can even run doc-tests
    /// assert_eq!(true, true);
    /// ```
    pub chan MyActor<MyError> {
        /// this function has no inputs or outputs -- not all that useful
        fn no_input_no_output() -> ();

        /// this function has one input and no outputs
        fn one_input_no_outpun(i: u32) -> ();

        /// this function has no inputs and one output
        fn no_input_one_output() -> u32;

        /// 1 and 1
        fn one_input_one_output(i: u32) -> u32;

        /// 2 and 2
        fn two_inputs_two_outputs(i: u32, j: u32) -> (u32, u32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ghost_actor::dependencies::futures::future::FutureExt;

    #[test]
    #[should_panic]
    fn test_mock_drop_panic() {
        // use a custom runtime so we don't break other test tasks
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async move {
            let mut m = MockMyActorHandler::new();
            m.expect_handle_one_input_one_output()
                .times(1)
                .returning(|x| Ok(async move { Ok(x + 1) }.boxed().into()));

            let _m = MockHandler::spawn(m, tokio::task::spawn).await;
            // the mock handler will be dropped here
            // the expected function will not have been run
            // this will cause the test to panic
        });
    }
}
