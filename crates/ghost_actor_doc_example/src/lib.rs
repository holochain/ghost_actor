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
