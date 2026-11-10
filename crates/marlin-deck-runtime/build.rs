fn main() {
    marlin_gerbil_aot_link::emit_deck_route_abi_link_directives();
    #[cfg(feature = "actor-runtime")]
    marlin_gerbil_aot_link::emit_deck_actor_link_directives();
}
