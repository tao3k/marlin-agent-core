use marlin_agent_environment::{RuntimeEnvironmentActivationRequest, RuntimeEnvironmentActivator};
use marlin_agent_test_support::{
    assert_custom_sub_agent_environment, assert_direnv_activation_fixture,
    assert_hook_environment_uses_root_home, custom_home_runtime_environment_fixture,
    direnv_activation_fixtures,
};

#[test]
fn custom_home_fixture_proves_hook_and_sub_agent_environment_boundaries() {
    let fixture = custom_home_runtime_environment_fixture();

    assert_hook_environment_uses_root_home(&fixture, fixture.hook_environment());
    assert_custom_sub_agent_environment(&fixture, fixture.sub_agent_environment());
}

#[tokio::test]
async fn direnv_activation_fixtures_cover_native_nix_and_instant_envrcs() {
    let fixtures = direnv_activation_fixtures();

    assert_eq!(fixtures.len(), 3);
    for fixture in fixtures {
        assert_eq!(
            fixture
                .envrc_file()
                .file_name()
                .and_then(|name| name.to_str()),
            Some(".envrc"),
            "{} must be backed by a concrete .envrc fixture",
            fixture.name()
        );
        assert!(
            !fixture.envrc_contents().trim().is_empty(),
            "{} must document the .envrc body used by the fixture",
            fixture.name()
        );

        let activator = RuntimeEnvironmentActivator::with_runner(fixture.runner());
        let result = activator
            .activate(RuntimeEnvironmentActivationRequest::new(
                fixture.environment().clone(),
                fixture.base_environment().clone(),
            ))
            .await;

        assert_direnv_activation_fixture(&fixture, &result);
    }
}
