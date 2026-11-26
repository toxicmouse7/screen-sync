use std::{
    ffi::c_void,
    io,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    ptr::{null, null_mut},
    sync::{
        Arc,
        mpsc::{self, Receiver, Sender},
    },
};

use eframe::egui::{self, ViewportCommand};
use tray_icon::{
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem},
};
use windows_sys::{
    Win32::{
        Foundation::{ERROR_ALREADY_EXISTS, GetLastError, HANDLE, INVALID_HANDLE_VALUE},
        System::Threading::{
            CreateEventW, INFINITE, RegisterWaitForSingleObject, SetEvent, UnregisterWaitEx,
        },
    },
    w,
};

use crate::i18n::{Key, Language};

pub struct Instance(OwnedHandle);

impl Instance {
    pub fn acquire() -> io::Result<Option<Self>> {
        // Later launches signal the first one even while its window is still being created.
        let event = unsafe { CreateEventW(null(), 0, 0, w!("Local\\ScreenSync.OpenWindow")) };
        let status = unsafe { GetLastError() };
        if event.is_null() {
            return Err(io::Error::from_raw_os_error(status as i32));
        }

        let event = unsafe { OwnedHandle::from_raw_handle(event) };
        if status == ERROR_ALREADY_EXISTS {
            if unsafe { SetEvent(event.as_raw_handle()) } == 0 {
                return Err(io::Error::last_os_error());
            }
            return Ok(None);
        }
        Ok(Some(Self(event)))
    }

    fn listen(&self, wake: Arc<WindowCommands>) -> io::Result<ActivationWait> {
        let mut wait = null_mut();
        if unsafe {
            RegisterWaitForSingleObject(
                &mut wait,
                self.0.as_raw_handle(),
                Some(activate),
                Arc::as_ptr(&wake).cast(),
                INFINITE,
                0,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(ActivationWait { wait, _wake: wake })
    }
}

struct ActivationWait {
    wait: HANDLE,
    _wake: Arc<WindowCommands>,
}

impl Drop for ActivationWait {
    fn drop(&mut self) {
        // Wait for the callback before releasing the state passed to Windows.
        unsafe { UnregisterWaitEx(self.wait, INVALID_HANDLE_VALUE) };
    }
}

unsafe extern "system" fn activate(state: *mut c_void, _timed_out: bool) {
    let wake = unsafe { &*state.cast::<WindowCommands>() };
    wake.send(Command::Open);
}

enum Command {
    Open,
    Exit,
}

struct WindowCommands {
    sender: Sender<Command>,
    ctx: egui::Context,
}

impl WindowCommands {
    fn send(&self, command: Command) {
        if self.sender.send(command).is_ok() {
            self.ctx.request_repaint();
        }
    }
}

pub struct Tray {
    _icon: TrayIcon,
    _activation: ActivationWait,
    open: MenuItem,
    exit: MenuItem,
    commands: Receiver<Command>,
    exiting: bool,
}

impl Tray {
    pub fn new(
        ctx: &egui::Context,
        instance: &Instance,
        language: Language,
    ) -> anyhow::Result<Self> {
        let (sender, commands) = mpsc::channel();
        let wake = Arc::new(WindowCommands {
            sender,
            ctx: ctx.clone(),
        });

        let menu = Menu::new();
        let open = MenuItem::new(language.text(Key::OpenWindow), true, None);
        let exit = MenuItem::new(language.text(Key::Exit), true, None);
        menu.append_items(&[&open, &exit])?;

        let open_id = open.id().clone();
        let exit_id = exit.id().clone();
        let menu_wake = Arc::clone(&wake);
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if event.id == open_id {
                menu_wake.send(Command::Open);
            } else if event.id == exit_id {
                menu_wake.send(Command::Exit);
            }
        }));

        let click_wake = Arc::clone(&wake);
        TrayIconEvent::set_event_handler(Some(move |event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                click_wake.send(Command::Open);
            }
        }));

        let icon = TrayIconBuilder::new()
            .with_icon(Icon::from_resource(1, None)?) // assets/windows.rc
            .with_tooltip("Screen Sync")
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .build()?;

        Ok(Self {
            _icon: icon,
            _activation: instance.listen(wake)?,
            open,
            exit,
            commands,
            exiting: false,
        })
    }

    pub fn set_language(&self, language: Language) {
        self.open.set_text(language.text(Key::OpenWindow));
        self.exit.set_text(language.text(Key::Exit));
    }

    pub fn request_exit(&mut self, ctx: &egui::Context) {
        self.exiting = true;
        ctx.send_viewport_cmd(ViewportCommand::Close);
    }

    pub fn update(&mut self, ctx: &egui::Context) {
        while let Ok(command) = self.commands.try_recv() {
            match command {
                Command::Open if !self.exiting => {
                    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
                    if ctx.input(|input| input.viewport().minimized == Some(true)) {
                        ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
                    }
                    ctx.send_viewport_cmd(ViewportCommand::Focus);
                }
                Command::Exit => self.request_exit(ctx),
                Command::Open => {}
            }
        }

        if !self.exiting && ctx.input(|input| input.viewport().close_requested()) {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(ViewportCommand::Visible(false));
        }
    }
}
