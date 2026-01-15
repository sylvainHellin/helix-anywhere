use crate::config::Config;
use crate::terminal::Terminal;
use anyhow::Result;
use cocoa::appkit::{
    NSApp, NSApplication, NSApplicationActivationPolicyAccessory, NSMenu, NSMenuItem,
    NSSquareStatusItemLength, NSStatusBar, NSStatusItem,
};
use cocoa::base::{id, nil, NO, YES};
use cocoa::foundation::{NSAutoreleasePool, NSSize, NSString};
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};
use std::sync::{Arc, Mutex};

// Embed the icon at compile time (36x36 for retina, will be displayed at 18x18 points)
// This is a template image: pure black pixels with alpha channel for shape
static ICON_DATA: &[u8] = include_bytes!("../assets/logo_app.png");

// Store config globally for menu callbacks
static mut GLOBAL_CONFIG: Option<Arc<Mutex<Config>>> = None;
static mut SAVE_CONFIG_CALLBACK: Option<Box<dyn Fn(&Config) + Send + Sync>> = None;
// Store the terminal submenu so we can update checkmarks
static mut TERMINAL_SUBMENU: Option<id> = None;

/// Initialize the menu bar app
pub fn init_app() {
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);

        // Initialize the application
        let app = NSApp();
        app.setActivationPolicy_(NSApplicationActivationPolicyAccessory);
    }
}

/// Create the status bar item with menu
pub fn create_status_item(config: Arc<Mutex<Config>>, on_save: impl Fn(&Config) + Send + Sync + 'static) -> Result<id> {
    unsafe {
        GLOBAL_CONFIG = Some(config.clone());
        SAVE_CONFIG_CALLBACK = Some(Box::new(on_save));

        let _pool = NSAutoreleasePool::new(nil);

        // Create status bar item
        let status_bar = NSStatusBar::systemStatusBar(nil);
        let status_item = status_bar.statusItemWithLength_(NSSquareStatusItemLength);

        // Set the button image (helix icon)
        let button: id = msg_send![status_item, button];

        // Try to load icon - first from embedded data, with fallback to text
        let image: id = {
            // Create NSData from embedded icon bytes
            let ns_data: id = msg_send![class!(NSData), dataWithBytes:ICON_DATA.as_ptr() length:ICON_DATA.len()];
            if ns_data == nil {
                log::warn!("Failed to create NSData");
                nil
            } else {
                // Create NSImage from data
                let img: id = msg_send![class!(NSImage), alloc];
                let img: id = msg_send![img, initWithData: ns_data];
                if img == nil {
                    log::warn!("Failed to create NSImage from data");
                }
                img
            }
        };

        if image != nil {
            // Set size (18x18 points for menu bar)
            let size = NSSize::new(18.0, 18.0);
            let _: () = msg_send![image, setSize: size];

            // Mark as template image for automatic dark/light mode handling
            // Template images should be black + alpha, system inverts as needed
            let _: () = msg_send![image, setTemplate: YES];

            let _: () = msg_send![button, setImage: image];
            log::info!("Menu bar icon loaded (template mode)");
        } else {
            // Fallback to text
            log::warn!("Using text fallback for menu bar");
            let title = NSString::alloc(nil).init_str("H");
            let _: () = msg_send![button, setTitle: title];
        }

        // Create menu
        let menu = NSMenu::new(nil).autorelease();

        // Add "About" item
        let about_title = NSString::alloc(nil).init_str("helix-anywhere v0.1.0");
        let about_item = NSMenuItem::alloc(nil)
            .initWithTitle_action_keyEquivalent_(about_title, Sel::from_ptr(std::ptr::null()), NSString::alloc(nil).init_str(""))
            .autorelease();
        let _: () = msg_send![about_item, setEnabled: NO];
        menu.addItem_(about_item);

        // Add separator
        let separator = NSMenuItem::separatorItem(nil);
        menu.addItem_(separator);

        // Add "Terminal" submenu
        let terminal_title = NSString::alloc(nil).init_str("Terminal");
        let terminal_item = NSMenuItem::alloc(nil)
            .initWithTitle_action_keyEquivalent_(terminal_title, Sel::from_ptr(std::ptr::null()), NSString::alloc(nil).init_str(""))
            .autorelease();

        let terminal_submenu = NSMenu::new(nil).autorelease();
        let terminal_submenu_title = NSString::alloc(nil).init_str("Terminal");
        let _: () = msg_send![terminal_submenu, setTitle: terminal_submenu_title];

        // Register the menu delegate class
        register_menu_delegate_class();

        // Add terminal options
        let current_terminal = {
            let cfg = config.lock().unwrap();
            cfg.terminal.name.clone()
        };

        // NSOnState = 1, NSOffState = 0
        const NS_ON_STATE: i64 = 1;
        const NS_OFF_STATE: i64 = 0;

        for terminal in Terminal::all() {
            let is_installed = terminal.is_installed();
            let is_current = terminal.config_name() == current_terminal;

            let item = if is_installed {
                let item_title = NSString::alloc(nil).init_str(terminal.display_name());
                let selector = sel!(selectTerminal:);
                let item = NSMenuItem::alloc(nil)
                    .initWithTitle_action_keyEquivalent_(item_title, selector, NSString::alloc(nil).init_str(""))
                    .autorelease();

                // Set checkmark state
                let state = if is_current { NS_ON_STATE } else { NS_OFF_STATE };
                let _: () = msg_send![item, setState: state];

                item
            } else {
                let disabled_name = format!("{} (not installed)", terminal.display_name());
                let disabled_title = NSString::alloc(nil).init_str(&disabled_name);
                let item = NSMenuItem::alloc(nil)
                    .initWithTitle_action_keyEquivalent_(disabled_title, Sel::from_ptr(std::ptr::null()), NSString::alloc(nil).init_str(""))
                    .autorelease();
                let _: () = msg_send![item, setEnabled: NO];
                item
            };

            // Store terminal name as represented object
            let terminal_name_str = NSString::alloc(nil).init_str(terminal.config_name());
            let _: () = msg_send![item, setRepresentedObject: terminal_name_str];

            // Set target to our delegate
            let delegate_class = Class::get("MenuDelegate").unwrap();
            let delegate: id = msg_send![delegate_class, new];
            let _: () = msg_send![item, setTarget: delegate];

            terminal_submenu.addItem_(item);
        }

        // Store submenu reference for later updates
        TERMINAL_SUBMENU = Some(terminal_submenu);

        let _: () = msg_send![terminal_item, setSubmenu: terminal_submenu];
        menu.addItem_(terminal_item);

        // Add separator
        let separator2 = NSMenuItem::separatorItem(nil);
        menu.addItem_(separator2);

        // Add "Quit" item
        let quit_title = NSString::alloc(nil).init_str("Quit");
        let quit_item = NSMenuItem::alloc(nil)
            .initWithTitle_action_keyEquivalent_(quit_title, sel!(terminate:), NSString::alloc(nil).init_str("q"))
            .autorelease();
        menu.addItem_(quit_item);

        // Set the menu
        status_item.setMenu_(menu);

        Ok(status_item)
    }
}

/// Register the Objective-C class for handling menu actions
fn register_menu_delegate_class() {
    let superclass = class!(NSObject);

    if Class::get("MenuDelegate").is_some() {
        return; // Already registered
    }

    let mut decl = ClassDecl::new("MenuDelegate", superclass).unwrap();

    // Add the selectTerminal: method
    extern "C" fn select_terminal(_this: &Object, _cmd: Sel, sender: id) {
        unsafe {
            // Get the represented object (terminal name)
            let represented_object: id = msg_send![sender, representedObject];
            if represented_object != nil {
                let terminal_name: *const i8 = msg_send![represented_object, UTF8String];
                let name = std::ffi::CStr::from_ptr(terminal_name)
                    .to_string_lossy()
                    .to_string();

                log::info!("Selected terminal: {}", name);

                // Update config
                if let Some(ref config) = GLOBAL_CONFIG {
                    let mut cfg = config.lock().unwrap();
                    cfg.terminal.name = name.clone();

                    // Save config
                    if let Some(ref save_fn) = SAVE_CONFIG_CALLBACK {
                        save_fn(&cfg);
                    }
                }

                // Update checkmarks in menu
                update_terminal_checkmarks(&name);
            }
        }
    }

    unsafe {
        decl.add_method(
            sel!(selectTerminal:),
            select_terminal as extern "C" fn(&Object, Sel, id),
        );
    }

    decl.register();
}

/// Run the application event loop
pub fn run_app() {
    unsafe {
        let app = NSApp();
        app.run();
    }
}

/// Update checkmarks in the terminal submenu
unsafe fn update_terminal_checkmarks(selected_name: &str) {
    const NS_ON_STATE: i64 = 1;
    const NS_OFF_STATE: i64 = 0;

    if let Some(submenu) = TERMINAL_SUBMENU {
        let count: i64 = msg_send![submenu, numberOfItems];
        for i in 0..count {
            let item: id = msg_send![submenu, itemAtIndex: i];
            if item == nil {
                continue;
            }

            // Get the represented object (terminal config name)
            let represented_object: id = msg_send![item, representedObject];
            if represented_object == nil {
                continue;
            }

            let terminal_name: *const i8 = msg_send![represented_object, UTF8String];
            if terminal_name.is_null() {
                continue;
            }

            let name = std::ffi::CStr::from_ptr(terminal_name)
                .to_string_lossy();

            // Set checkmark state
            let state = if name == selected_name {
                NS_ON_STATE
            } else {
                NS_OFF_STATE
            };
            let _: () = msg_send![item, setState: state];
        }
    }
}
