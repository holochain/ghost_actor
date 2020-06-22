# ghost_actor changelog

## 0.2.0

- [#25](https://github.com/holochain/ghost_actor/pull/25) - doc filename typo
- [#24](https://github.com/holochain/ghost_actor/pull/24) - internal sender pattern example and docs
- [#23](https://github.com/holochain/ghost_actor/pull/23) - batch handler processing / shutdown fix / error cleanup / attach receiver
- [#22](https://github.com/holochain/ghost_actor/pull/22)
The spawn logic has been rewritten into a `GhostActorBuilder` concept that is capable of connecting any number of `GhostEvent` Receivers into a `stream_multiplexer` and forwarding those events to a single handler within an actor task.

This does away with the explicit "`ghost_actor_custom`" and "`ghost_actor_interna`l" concepts, but still allows you to accomplish those by simply creating more event types and integrating those into your handler. I.e. for an "internal" sender, simply don't expose that sender in your spawn function.

This also implements issue #21 making the sender functions `&self` and thus easier to work with.
- [#19](https://github.com/holochain/ghost_actor/pull/19) - implement PartialEq, Eq, and Hash for Senders - allowing distinguishing distinct actors from the sender side.
- [#18](https://github.com/holochain/ghost_actor/pull/18) - Upgrade `must_future` + remove the Box requirement on the spawn callback.
- [#17](https://github.com/holochain/ghost_actor/pull/17) - `handle_ghost_actor_shutdown` handler function is invoked when the actor loop stops / is stopped.
- [#16](https://github.com/holochain/ghost_actor/pull/16) - `ghost_chan!` generates handler trait + dispatch helper.
- [#15](https://github.com/holochain/ghost_actor/pull/15) - Respond callbacks are structs instead of dyn closures.

## 0.1.0

- Initial release of `macro_rules!` macro + futures ghost actors.

## pre 0.1.0

- Dead-end experiment with proc-macro / non-futures code.
