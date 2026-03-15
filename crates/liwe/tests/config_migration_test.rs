use indoc::indoc;
use liwe::model::config::{migrate_v2_to_v3, ActionDefinition, Configuration};

#[test]
fn test_config_without_version_gets_parsed_correctly() {
    let config_str = indoc! {r#"
        [library]
        path = ""
        date_format = "%Y-%m-%d"

        [markdown]
        refs_extension = ""

        [commands]

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

        [commands]

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

        [commands]

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
fn test_template_configuration_has_version_3() {
    let config = Configuration::template();
    assert_eq!(config.version, Some(3));

    assert!(!config.actions.is_empty());
    assert!(config.actions.contains_key("extract"));
    assert!(config.actions.contains_key("extract_all"));
    assert!(config.actions.contains_key("link"));
}

#[test]
fn test_migrate_v2_to_v3_renames_models_to_commands() {
    let v2_config = indoc! {r#"
        version = 2

        [library]
        path = ""
        date_format = "%Y-%m-%d"

        [markdown]
        refs_extension = ""

        [models.default]
        api_key_env = "OPENAI_API_KEY"
        base_url = "https://api.openai.com"
        name = "gpt-4o"

        [models.fast]
        api_key_env = "OPENAI_API_KEY"
        base_url = "https://api.openai.com"
        name = "gpt-4o-mini"

        [actions]
    "#};

    let migrated = migrate_v2_to_v3(v2_config);

    assert!(migrated.contains("[commands.default]"));
    assert!(migrated.contains("[commands.fast]"));
    assert!(migrated.contains("run = \"\""));
    assert!(!migrated.contains("[models.default]"));
    assert!(!migrated.contains("[models.fast]"));
}

#[test]
fn test_migrate_v2_to_v3_renames_transform_action_fields() {
    let v2_config = indoc! {r#"
        version = 2

        [library]
        path = ""

        [markdown]
        refs_extension = ""

        [models.default]
        api_key_env = "OPENAI_API_KEY"
        base_url = "https://api.openai.com"
        name = "gpt-4o"

        [actions.rewrite]
        type = "transform"
        title = "Rewrite"
        model = "default"
        prompt_template = "Rewrite this: {{context}}"
        context = "Document"
    "#};

    let migrated = migrate_v2_to_v3(v2_config);

    assert!(migrated.contains("command = \"default\""));
    assert!(migrated.contains("input_template = \"Rewrite this: {{context}}\""));
    assert!(!migrated.contains("model = \"default\""));
    assert!(!migrated.contains("prompt_template"));
    assert!(!migrated.contains("context = \"Document\""));
}

#[test]
fn test_migrate_v2_to_v3_preserves_non_transform_actions() {
    let v2_config = indoc! {r##"
        version = 2

        [library]
        path = ""

        [markdown]
        refs_extension = ""

        [models]

        [actions.extract]
        type = "extract"
        title = "Extract"
        key_template = "{{id}}"
        link_type = "markdown"

        [actions.attach]
        type = "attach"
        title = "Add Date"
        key_template = "{{today}}"
        document_template = "# {{today}}"
    "##};

    let migrated = migrate_v2_to_v3(v2_config);

    assert!(migrated.contains("[actions.extract]"));
    assert!(migrated.contains("type = \"extract\""));
    assert!(migrated.contains("title = \"Extract\""));
    assert!(migrated.contains("key_template = \"{{id}}\""));

    assert!(migrated.contains("[actions.attach]"));
    assert!(migrated.contains("type = \"attach\""));
    assert!(migrated.contains("document_template = \"# {{today}}\""));
}

#[test]
fn test_migrated_v2_config_parses_correctly() {
    let v2_config = indoc! {r#"
        version = 2

        [library]
        path = ""
        date_format = "%Y-%m-%d"

        [markdown]
        refs_extension = ""

        [models.default]
        api_key_env = "OPENAI_API_KEY"
        base_url = "https://api.openai.com"
        name = "gpt-4o"

        [actions.rewrite]
        type = "transform"
        title = "Rewrite"
        model = "default"
        prompt_template = "Rewrite this: {{context}}"
        context = "Document"

        [actions.extract]
        type = "extract"
        title = "Extract"
        key_template = "{{id}}"
        link_type = "markdown"
    "#};

    let migrated = migrate_v2_to_v3(v2_config);

    let parsed: Configuration =
        toml::from_str(&migrated).expect("Failed to parse migrated config");

    assert!(parsed.commands.contains_key("default"));
    assert_eq!(parsed.commands.get("default").unwrap().run, "");

    assert!(parsed.actions.contains_key("rewrite"));
    if let Some(ActionDefinition::Transform(transform)) = parsed.actions.get("rewrite") {
        assert_eq!(transform.title, "Rewrite");
        assert_eq!(transform.command, "default");
        assert_eq!(transform.input_template, "Rewrite this: {{context}}");
    } else {
        panic!("rewrite should be Transform type");
    }

    assert!(parsed.actions.contains_key("extract"));
    if let Some(ActionDefinition::Extract(extract)) = parsed.actions.get("extract") {
        assert_eq!(extract.title, "Extract");
        assert_eq!(extract.key_template, "{{id}}");
    } else {
        panic!("extract should be Extract type");
    }
}
