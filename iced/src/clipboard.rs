
#[cfg(not(target_family = "wasm"))]
pub use iced_winit::Clipboard;


#[cfg(target_family = "wasm")]
mod web_clipboard {

    use platform::{AppCtx, web_clipboard::WebClipboard};
    use iced_winit::core::clipboard::{Clipboard as IcedClipboard, Kind};

    #[derive(Debug)]
    pub struct Clipboard {
        pub web: WebClipboard,
    }

    impl Clipboard {
        pub fn connect(app_ctx: &AppCtx) -> Self {
            Self { web: WebClipboard::connect(app_ctx, true) } // listen to canvas events
        }
    }

    // make usable with iced
    impl IcedClipboard for Clipboard {
        fn read(&self, _kind: Kind) -> Option<String> { self.web.read() }
        fn write(&mut self, _kind: Kind, text: String) { self.web.write(text) }
    }
}


#[cfg(target_family = "wasm")]
pub use web_clipboard::Clipboard;
