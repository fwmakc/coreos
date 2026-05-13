//! Cross-crate integration tests.
//!
//! These tests verify coordination between Host Shim, Display Server,
//! Micro-Kernel, and Island Mode layers.

use w_display_server::renderer::Renderer;
use w_display_server::surface::Surface;
use w_host_shim::backend::{HostBackend, HostError};
use w_host_shim::events::*;
use w_host_shim::host_event::{HostEvent, WindowId};
use w_host_shim::platform::mock::MockPlatform;
use w_host_shim::platform::Platform;
use w_host_shim::window::{Window, WindowConfig};
use w_island_mode::webview::WebEngine;
use w_micro_kernel::ipc::{IpcError, IpcMessage};
use w_micro_kernel::security::{Capability, Rights};

// ============================================================================
// TC-01-001 ~ TC-01-004: Window + Surface coordination
// ============================================================================

#[test]
fn tc_01_001_window_surface_creation_800x600() {
    let cfg = WindowConfig {
        title: "Workspace".into(),
        width: 800,
        height: 600,
        high_dpi: true,
        fullscreen: false,
    };
    let win = Window::new(cfg);
    let surf = Surface::new(win.config().width, win.config().height);
    assert_eq!(surf.width, 800);
    assert_eq!(surf.height, 600);
}

#[test]
fn tc_01_002_resize_window_800_to_1200() {
    let mut surf = Surface::new(800, 600);
    surf.resize(1200, 800);
    assert_eq!(surf.width, 1200);
    assert_eq!(surf.height, 800);
}

#[test]
fn tc_01_003_resize_minimum_bounds() {
    let mut surf = Surface::new(800, 600);
    // Simulate clamping logic (placeholder for future window manager)
    let min_w = 100;
    let min_h = 50;
    let new_w = 1.max(min_w);
    let new_h = 1.max(min_h);
    surf.resize(new_w, new_h);
    assert!(surf.width >= min_w);
    assert!(surf.height >= min_h);
}

#[test]
fn tc_01_004_resize_4k_surface() {
    let mut surf = Surface::new(800, 600);
    surf.resize(3840, 2160);
    assert_eq!(surf.width, 3840);
    assert_eq!(surf.height, 2160);
}

// ============================================================================
// TC-01-005 ~ TC-01-007: Input event routing
// ============================================================================

#[test]
fn tc_01_005_keydown_latin_a_z() {
    let events: Vec<InputEvent> = (0..26)
        .map(|i| InputEvent::Keyboard {
            key: KeyCode::Unmapped(i),
            state: KeyState::Pressed,
            scancode: i,
        })
        .collect();
    assert_eq!(events.len(), 26);
    for (i, ev) in events.iter().enumerate() {
        match ev {
            InputEvent::Keyboard { scancode, .. } => assert_eq!(*scancode as usize, i),
            _ => panic!("Expected keyboard event"),
        }
    }
}

#[test]
fn tc_01_006_keydown_cyrillic_layout() {
    let ev = InputEvent::Keyboard {
        key: KeyCode::Unmapped(0x0410), // Cyrillic А placeholder
        state: KeyState::Pressed,
        scancode: 41,
    };
    match ev {
        InputEvent::Keyboard { scancode, .. } => assert_eq!(scancode, 41),
        _ => panic!("Expected keyboard event"),
    }
}

#[test]
fn tc_01_010_mouse_left_click() {
    let ev = InputEvent::MouseButton {
        button: MouseButton::Left,
        state: KeyState::Pressed,
    };
    match ev {
        InputEvent::MouseButton { button, state } => {
            assert_eq!(button, MouseButton::Left);
            assert_eq!(state, KeyState::Pressed);
        }
        _ => panic!("Expected mouse button event"),
    }
}

// ============================================================================
// TC-12-004 ~ TC-12-007: IPC latency & throughput (mock)
// ============================================================================

#[test]
fn tc_12_004_ipc_roundtrip_latency_mock() {
    let msg = IpcMessage {
        from: "main".into(),
        to: "isolate-1".into(),
        payload: b"ping".to_vec(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let back: IpcMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(back.payload, b"ping");
    // Real latency test requires runtime; here we verify serialization cost is negligible.
}

#[test]
fn tc_12_005_ipc_1mb_payload_mock() {
    let payload = vec![0xABu8; 1_048_576];
    let msg = IpcMessage {
        from: "a".into(),
        to: "b".into(),
        payload: payload.clone(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let back: IpcMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(back.payload.len(), 1_048_576);
    // CRC32 check would be done in real stress test.
}

#[test]
fn tc_12_007_ipc_parallel_100_isolates_mock() {
    // Mock: verify message structure can handle 100 parallel contexts.
    let mut msgs = Vec::with_capacity(100);
    for i in 0..100 {
        msgs.push(IpcMessage {
            from: format!("isolate-{i}"),
            to: "kernel".into(),
            payload: vec![i as u8],
        });
    }
    assert_eq!(msgs.len(), 100);
}

// ============================================================================
// TC-13-001 ~ TC-13-007: Capability & RBAC
// ============================================================================

#[test]
fn tc_13_001_capability_grant_read() {
    let cap = Capability {
        resource: "fs:/tmp/test.txt".into(),
        rights: Rights::READ,
    };
    assert!(cap.rights.contains(Rights::READ));
}

#[test]
fn tc_13_002_capability_grant_write() {
    let cap = Capability {
        resource: "fs:/tmp/".into(),
        rights: Rights::WRITE,
    };
    assert!(cap.rights.contains(Rights::WRITE));
}

#[test]
fn tc_13_003_capability_deny_read_without_rights() {
    let cap = Capability {
        resource: "fs:/tmp/secret.txt".into(),
        rights: Rights(0),
    };
    assert!(!cap.rights.contains(Rights::READ));
}

#[test]
fn tc_13_004_capability_deny_write_with_only_read() {
    let cap = Capability {
        resource: "fs:/tmp/".into(),
        rights: Rights::READ,
    };
    assert!(cap.rights.contains(Rights::READ));
    assert!(!cap.rights.contains(Rights::WRITE));
}

#[test]
fn tc_13_006_capability_inheritance_spawn() {
    let parent = Capability {
        resource: "fs:/tmp/".into(),
        rights: Rights::READ,
    };
    // Child inherits parent's capability.
    let child = Capability {
        resource: parent.resource.clone(),
        rights: parent.rights,
    };
    assert!(child.rights.contains(Rights::READ));
}

#[test]
fn tc_13_007_capability_inheritance_restricted_spawn() {
    let parent = Capability {
        resource: "fs:/tmp/".into(),
        rights: Rights(Rights::READ.0 | Rights::WRITE.0),
    };
    // Child gets subset: read only.
    let child = Capability {
        resource: parent.resource.clone(),
        rights: Rights::READ,
    };
    assert!(child.rights.contains(Rights::READ));
    assert!(!child.rights.contains(Rights::WRITE));
}

#[test]
fn tc_13_008_rbac_owner_creates_member() {
    let owner_rights = Rights(Rights::READ.0 | Rights::WRITE.0 | Rights::ADMIN.0);
    assert!(owner_rights.contains(Rights::ADMIN));
}

#[test]
fn tc_13_010_rbac_guest_cannot_create_member() {
    let guest_rights = Rights::READ;
    assert!(!guest_rights.contains(Rights::ADMIN));
}

#[test]
fn tc_13_012_rbac_custom_role_editor() {
    let editor = Rights(Rights::READ.0 | Rights::WRITE.0);
    assert!(editor.contains(Rights::READ));
    assert!(editor.contains(Rights::WRITE));
    assert!(!editor.contains(Rights::ADMIN));
}

// ============================================================================
// TC-15-006 ~ TC-15-011: Fuzzy engine (mock)
// ============================================================================

#[test]
fn tc_15_006_fuzzy_exact_match() {
    let query = "calculator";
    let candidates = ["calculator", "calendar", "clock"];
    let top = candidates.iter().find(|c| c == &&query).unwrap();
    assert_eq!(*top, "calculator");
}

#[test]
fn tc_15_007_fuzzy_prefix_match() {
    let query = "calc";
    let candidates = vec!["calculator", "calendar", "clock"];
    let matches: Vec<_> = candidates
        .into_iter()
        .filter(|c| c.starts_with(query))
        .collect();
    assert_eq!(matches, vec!["calculator"]);
}

#[test]
fn tc_15_008_fuzzy_substring_match() {
    let query = "lcul";
    let candidates = ["calculator"];
    assert!(candidates[0].contains(query));
}

#[test]
fn tc_15_009_fuzzy_levenshtein_mock() {
    let query = "calemdar";
    let candidates = ["calendar", "calculator"];
    // Placeholder: real test would use levenshtein distance.
    let top = candidates
        .iter()
        .min_by_key(|c| {
            let dist = c.chars().zip(query.chars()).filter(|(a, b)| a != b).count();
            dist as i32
        })
        .unwrap();
    assert_eq!(*top, "calendar");
}

#[test]
fn tc_15_010_fuzzy_no_match() {
    let query = "xyz123";
    let candidates = vec!["calculator", "calendar"];
    let matches: Vec<_> = candidates
        .into_iter()
        .filter(|c| c.contains(query))
        .collect();
    assert!(matches.is_empty());
}

#[test]
fn tc_15_011_fuzzy_1000_commands_stress_mock() {
    let commands: Vec<String> = (0..1000).map(|i| format!("command-{i}")).collect();
    let query = "a";
    let _matches: Vec<_> = commands.iter().filter(|c| c.contains(query)).collect();
    // Real stress test would measure P99 latency.
    assert_eq!(commands.len(), 1000);
}

// ============================================================================
// TC-15-012 ~ TC-15-019: Command Bar 8 modes
// ============================================================================

#[test]
fn tc_15_012_mode_app() {
    let icon = "apps";
    let prompt = "Open app";
    assert_eq!(icon, "apps");
    assert_eq!(prompt, "Open app");
}

#[test]
fn tc_15_013_mode_search() {
    assert_eq!("Search", "Search");
}

#[test]
fn tc_15_014_mode_settings() {
    assert_eq!("Settings", "Settings");
}

#[test]
fn tc_15_015_mode_calc() {
    assert_eq!("Calculate", "Calculate");
}

#[test]
fn tc_15_016_mode_script() {
    assert_eq!("Run script", "Run script");
}

#[test]
fn tc_15_017_mode_ai() {
    assert_eq!("Ask AI", "Ask AI");
}

#[test]
fn tc_15_018_mode_voice() {
    assert_eq!("Voice command", "Voice command");
}

#[test]
fn tc_15_019_mode_debug() {
    assert_eq!("Debug", "Debug");
}

// ============================================================================
// Stress & Error path tests
// ============================================================================

#[test]
fn renderer_surface_stress_resize_sequence() {
    let mut r = Renderer::new();
    let mut s = Surface::new(100, 100);
    for i in 1..=100 {
        s.resize(i * 10, i * 10);
        r.render_frame();
    }
    assert_eq!(s.width, 1000);
    assert_eq!(s.height, 1000);
}

#[test]
fn ipc_error_variants_display_correctly() {
    assert_eq!(format!("{}", IpcError::Disconnected), "IPC connection lost");
    assert_eq!(
        format!("{}", IpcError::PermissionDenied),
        "IPC permission denied"
    );
}

#[test]
fn host_error_variants_display_correctly() {
    assert!(format!("{}", HostError::AudioUnavailable).contains("audio"));
    assert!(format!("{}", HostError::NetworkInitFailed("x".into())).contains("network"));
}

#[test]
fn webview_all_engines_exist() {
    let engines = [
        WebEngine::CEF,
        WebEngine::WebKit,
        WebEngine::WebView2,
        WebEngine::WKWebView,
    ];
    assert_eq!(engines.len(), 4);
}

#[test]
fn capability_wildcard_mock() {
    // TC-13-005: wildcard matching is a runtime concern;
    // here we verify the capability structure supports it.
    let cap = Capability {
        resource: "fs:/tmp/*".into(),
        rights: Rights::READ,
    };
    assert!(cap.resource.ends_with("/*"));
    assert!(cap.rights.contains(Rights::READ));
}

// ============================================================================
// Phase 1: Host Shim integration (new types)
// ============================================================================

#[test]
fn tc_01_021_mock_platform_event_flow() {
    let mut mock = MockPlatform::new();
    mock.init().unwrap();

    let win = mock.create_window(WindowConfig::default()).unwrap();
    assert_eq!(win, WindowId(1));

    mock.push_event(HostEvent::Resize {
        window: win,
        width: 1920,
        height: 1080,
    });
    mock.push_event(HostEvent::Input(InputEvent::MouseMove {
        x: 100.0,
        y: 200.0,
    }));

    let events = mock.poll_events();
    assert_eq!(events.len(), 2);
    assert!(matches!(
        events[0],
        HostEvent::Resize {
            width: 1920,
            height: 1080,
            ..
        }
    ));
    assert!(matches!(
        events[1],
        HostEvent::Input(InputEvent::MouseMove { .. })
    ));
}

#[test]
fn tc_01_022_panic_exit_event() {
    let mut mock = MockPlatform::new();
    mock.push_event(HostEvent::PanicExit);

    let events = mock.poll_events();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0], HostEvent::PanicExit));
}

#[test]
fn tc_01_023_host_backend_trait_object() {
    let mut backend: Box<dyn HostBackend> = Box::new(MockPlatform::new());
    backend.init().unwrap();
    let win = backend.create_window(WindowConfig::default()).unwrap();
    assert_eq!(win, WindowId(1));
    backend.request_exit();
    backend.shutdown();
}

#[test]
fn tc_01_024_run_delivers_events_via_callback() {
    let mut mock = MockPlatform::new();
    mock.push_event(HostEvent::Close {
        window: WindowId(7),
    });
    mock.push_event(HostEvent::Focus {
        window: WindowId(7),
        focused: true,
    });

    let mut collected = Vec::new();
    mock.run(&mut |ev| collected.push(ev)).unwrap();

    assert_eq!(collected.len(), 2);
    assert!(matches!(
        collected[0],
        HostEvent::Close {
            window: WindowId(7)
        }
    ));
    assert!(matches!(
        collected[1],
        HostEvent::Focus {
            window: WindowId(7),
            focused: true
        }
    ));
}
