use cunzhi::config::{AppState, load_config_and_apply_window_settings, load_standalone_telegram_config};
use cunzhi::utils::auto_init_logger;
use cunzhi::log_important;
use anyhow::Result;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::Manager;

// 重新导出所有命令函数，避免重名冲突
pub use cunzhi::mcp::commands::*;
pub use cunzhi::telegram::commands::*;
pub use cunzhi::telegram::handle_telegram_only_mcp_request;
pub use cunzhi::ui::{commands::*, window::*, audio::*, audio_assets::*, updater::*};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState::default())
        .manage(AudioController {
            should_stop: Arc::new(AtomicBool::new(false)),
        })
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            get_always_on_top,
            set_always_on_top,
            sync_window_state,
            get_audio_notification_enabled,
            set_audio_notification_enabled,
            get_audio_url,
            set_audio_url,
            play_notification_sound,
            test_audio_sound,
            stop_audio_sound,
            get_available_audio_assets,
            refresh_audio_assets,
            get_theme,
            set_theme,
            get_window_config,
            set_window_config,
            get_reply_config,
            set_reply_config,
            get_window_settings,
            set_window_settings,
            get_window_settings_for_mode,
            get_window_constraints_cmd,
            get_current_window_size,
            apply_window_constraints,
            update_window_size,
            get_mcp_tools_config,
            set_mcp_tool_enabled,
            get_mcp_tools_status,
            reset_mcp_tools_config,
            send_mcp_response,
            get_cli_args,
            read_mcp_request,
            select_image_files,
            get_telegram_config,
            set_telegram_config,
            test_telegram_connection_cmd,
            auto_get_chat_id,
            start_telegram_sync,
            open_external_url,
            exit_app,
            build_mcp_send_response,
            build_mcp_continue_response,
            check_for_updates,
            download_and_install_update,
            get_current_version,
            restart_app
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();

            // 在setup中直接调用，不使用spawn
            tauri::async_runtime::block_on(async {
                let state = app.state::<AppState>();
                if let Err(e) = load_config_and_apply_window_settings(&state, &app_handle).await {
                    log_important!(warn, "加载配置失败: {}", e);
                }

                // 初始化音频资源管理器
                if let Err(e) = initialize_audio_asset_manager(&app_handle) {
                    log_important!(warn, "初始化音频资源管理器失败: {}", e);
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() -> Result<()> {
    // 初始化日志系统
    if let Err(e) = auto_init_logger() {
        // 日志系统初始化失败时，只能使用 stderr 输出错误
        // 这个错误很少发生，且不会影响 MCP 响应（因为日志系统会自动处理 MCP 模式）
        eprintln!("初始化日志系统失败: {}", e);
    }

    // 检查程序是如何被调用的
    let args: Vec<String> = std::env::args().collect();

    // 处理命令行参数
    if args.len() >= 3 && args[1] == "--mcp-request" {
        // MCP 请求模式：检查是否需要启动GUI
        let request_file = &args[2];

        // 检查Telegram配置，决定是否启用纯Telegram模式
        match load_standalone_telegram_config() {
            Ok(telegram_config) => {
                if telegram_config.enabled && telegram_config.hide_frontend_popup {
                    // 纯Telegram模式：不启动GUI，直接处理
                    if let Err(e) = tokio::runtime::Runtime::new()
                        .unwrap()
                        .block_on(handle_telegram_only_mcp_request(request_file))
                    {
                        log_important!(error, "处理Telegram请求失败: {}", e);
                        std::process::exit(1);
                    }
                    return Ok(());
                } else {
                    // 正常模式：启动GUI处理弹窗
                    run();
                }
            }
            Err(e) => {
                log_important!(warn, "加载Telegram配置失败: {}，使用默认GUI模式", e);
                // 配置加载失败时，使用默认行为（启动GUI）
                run();
            }
        }
    } else if args.len() >= 2 && (args[1] == "--help" || args[1] == "-h") {
        // 显示帮助信息
        print_help();
    } else if args.len() >= 2 && (args[1] == "--version" || args[1] == "-v") {
        // 显示版本信息
        print_version();
    } else {
        // 正常启动 GUI 应用（设置界面）
        run();
    }

    Ok(())
}

/// 显示帮助信息
fn print_help() {
    println!("寸止 - 智能代码审查工具");
    println!();
    println!("用法:");
    println!("  等一下                    启动设置界面");
    println!("  等一下 --mcp-request <文件>  处理 MCP 请求");
    println!("  等一下 --help             显示此帮助信息");
    println!("  等一下 --version          显示版本信息");
}

/// 显示版本信息
fn print_version() {
    println!("寸止 v{}", env!("CARGO_PKG_VERSION"));
}


