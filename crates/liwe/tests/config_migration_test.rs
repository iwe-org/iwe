use indoc::indoc;
use liwe::model::config::Configuration;

#[test]
fn test_config_without_version_gets_parsed_correctly() {
    let config_str = indoc! {r#"
        [library]
        path = ""
        date_format = "%Y-%m-%d"

        [markdown]
        refs_extension = ""

        [models]

        [actions]
    "#};

    let parsed: Configuration = toml::from_str(config_str).expect("Failed to parse config");

    assert_eq!(parsed.version, None);
    assert_eq!(parsed.actions.len(), 0);
}

#[test]
fn test_config_with_version_1_parses_correctly() {
    let config_str = indoc! {r#"
        version = 1

        [library]
        path = ""
        date_format = "%Y-%m-%d"

        [markdown]
        refs_extension = ""

        [models]

        [actions]
    "#};

    let parsed: Configuration =
        toml::from_str(config_str).expect("Failed to parse config with version");

    assert_eq!(parsed.version, Some(1));
}

#[test]
fn test_config_with_extract_action_parses_correctly() {
    let config_str = indoc! {r#"
        [library]
        path = ""
        date_format = "%Y-%m-%d"

        [markdown]
        refs_extension = ""

        [models]

        [actions]

        [actions.my_extract]
        type = "extract"
        title = "My Extract"
        key_template = "{{id}}"
        link_type = "markdown"
    "#};

    let parsed: Configuration =
        toml::from_str(config_str).expect("Failed to parse config with extract action");

    assert_eq!(parsed.actions.len(), 1);
    assert!(parsed.actions.contains_key("my_extract"));

    if let Some(liwe::model::config::ActionDefinition::Extract(extract)) =
        parsed.actions.get("my_extract")
    {
        assert_eq!(extract.title, "My Extract");
        assert_eq!(extract.key_template, "{{id}}");
    } else {
        panic!("my_extract should be Extract type");
    }
}

#[test]
fn test_default_configuration_has_version_1() {
    let config = Configuration::default();
    assert_eq!(config.version, Some(1));
}

#[test]
fn test_template_configuration_has_version_2() {
    let config = Configuration::template();
    assert_eq!(config.version, Some(2));

    assert!(config.actions.len() > 0);
    assert!(config.actions.contains_key("extract"));
    assert!(config.actions.contains_key("extract_all"));
    assert!(config.actions.contains_key("link"));
}
