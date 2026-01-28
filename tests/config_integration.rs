//! Integration tests for configuration loading
//!
//! Tests that verify config loading from files and environment variables.

use rust4d::config::AppConfig;
use serial_test::serial;

#[test]
#[serial]
fn test_env_override() {
    std::env::set_var("R4D_WINDOW__TITLE", "Test From Env");
    let config = AppConfig::load().unwrap();
    println!("Window title: {}", config.window.title);
    assert_eq!(config.window.title, "Test From Env");
    std::env::remove_var("R4D_WINDOW__TITLE");
}

#[test]
#[serial]
fn test_user_config_loading() {
    // Remove env var to test file-based config
    std::env::remove_var("R4D_WINDOW__TITLE");

    // Debug: print current dir and check if files exist
    let cwd = std::env::current_dir().unwrap();
    println!("Current dir: {:?}", cwd);
    println!(
        "config/default.toml exists: {}",
        cwd.join("config/default.toml").exists()
    );
    println!(
        "config/user.toml exists: {}",
        cwd.join("config/user.toml").exists()
    );

    let config = AppConfig::load().unwrap();
    println!("Window title from file: {}", config.window.title);
}
