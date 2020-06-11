# ghost_actor changelog

## 0.2.0

- [#18](https://github.com/holochain/ghost_actor/pull/18) - Upgrade must_future + remove the Box requirement on the spawn callback.
- [#17](https://github.com/holochain/ghost_actor/pull/17) - `handle_ghost_actor_shutdown` handler function is invoked when the actor loop stops / is stopped.
- [#16](https://github.com/holochain/ghost_actor/pull/16) - `ghost_chan!` generates handler trait + dispatch helper.
- [#15](https://github.com/holochain/ghost_actor/pull/15) - Respond callbacks are structs instead of dyn closures.

## 0.1.0

- Initial release of `macro_rules!` macro + futures ghost actors.

## pre 0.1.0

- Dead-end experiment with proc-macro / non-futures code.
