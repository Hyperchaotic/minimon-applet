use zbus::{blocking::Connection, blocking::Proxy};
use zvariant::OwnedFd;

pub const INHIBITOR_OPTIONS: [&str; 4] = [" 15 min ", " 30 min ", " 60 min ", " Infinite "];

pub struct SleepAndScreenInhibitor {
    connection: Connection,
    sleep_fd: Option<OwnedFd>,
    screensaver_cookie: Option<u32>,
}

impl SleepAndScreenInhibitor {
    pub fn new() -> zbus::Result<Self> {
        let connection = Connection::session()?; // For screensaver
        Ok(Self {
            connection,
            sleep_fd: None,
            screensaver_cookie: None,
        })
    }

    /// Inhibit system sleep and screen dimming
    pub fn inhibit(&mut self, app_name: &str, reason: &str) -> zbus::Result<()> {
        // Inhibit system sleep via system bus
        let sys_conn = Connection::system()?;
        let login_proxy = Proxy::new(
            &sys_conn,
            "org.freedesktop.login1",
            "/org/freedesktop/login1",
            "org.freedesktop.login1.Manager",
        )?;

        let sleep_fd = login_proxy
            .call_method("Inhibit", &("sleep", app_name, reason, "block"))?
            .body::<zbus::zvariant::OwnedFd>()?;

        self.sleep_fd = Some(sleep_fd);

        // Inhibit screen dimming via session bus
        let screensaver_proxy = Proxy::new(
            &self.connection,
            "org.freedesktop.ScreenSaver",
            "/org/freedesktop/ScreenSaver",
            "org.freedesktop.ScreenSaver",
        )?;

        let cookie = screensaver_proxy
            .call_method("Inhibit", &(app_name, reason))?
            .body::<u32>()?;

        self.screensaver_cookie = Some(cookie);

        Ok(())
    }

    /// Uninhibit both sleep and screen
    pub fn uninhibit(&mut self) {
        self.sleep_fd = None; // Drop the fd

        if let Some(cookie) = self.screensaver_cookie.take() {
            if let Ok(proxy) = Proxy::new(
                &self.connection,
                "org.freedesktop.ScreenSaver",
                "/org/freedesktop/ScreenSaver",
                "org.freedesktop.ScreenSaver",
            ) {
                let _ = proxy.call_method("UnInhibit", &(cookie));
            }
        }
    }

    pub fn is_inhibiting(&self) -> bool {
        self.sleep_fd.is_some() || self.screensaver_cookie.is_some()
    }
}
