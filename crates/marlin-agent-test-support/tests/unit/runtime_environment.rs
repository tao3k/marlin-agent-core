use marlin_agent_test_support::{
    assert_custom_sub_agent_environment, assert_hook_environment_uses_root_home,
    custom_home_runtime_environment_fixture,
};

#[test]
fn custom_home_fixture_proves_hook_and_sub_agent_environment_boundaries() {
    let fixture = custom_home_runtime_environment_fixture();

    assert_hook_environment_uses_root_home(&fixture, fixture.hook_environment());
    assert_custom_sub_agent_environment(&fixture, fixture.sub_agent_environment());
}
