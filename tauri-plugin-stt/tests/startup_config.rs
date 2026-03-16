use tauri_plugin_stt::Config;

#[test]
fn config_default_is_tiny_en() {
    let config = Config::default();
    assert_eq!(config.model_id, Some("tiny.en".to_string()));
}

#[test]
fn config_deserializes_camel_case() {
    let json = r#"{"modelId": "base"}"#;
    let config: Config = serde_json::from_str(json).expect("should deserialize");
    assert_eq!(config.model_id, Some("base".to_string()));
}

#[test]
fn config_deserializes_empty_as_none() {
    let json = r#"{}"#;
    let config: Config = serde_json::from_str(json).expect("should deserialize");
    assert_eq!(config.model_id, None);
}
