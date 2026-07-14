//! Runtime-owned host and user identity.
//!
//! Portable dumps preserve Lisp state from the build process, but identity
//! values describe the process that loaded the image.  This module is the one
//! lifecycle seam that captures those values, installs their Lisp variables,
//! and tracks GNU's owned-versus-overridden `system-name` distinction.

use super::eval::Context;
use super::value::{Value, eq_value};

#[derive(Clone, Debug)]
pub(crate) struct PasswdEntry {
    pub(crate) login: String,
    pub(crate) gecos: String,
}

std::cfg_select! {
    unix => {
        use std::ffi::{CStr, CString};

        const SECONDARY_LOGIN_ENV: &str = "USER";

        fn passwd_entry_from_raw(passwd: &libc::passwd) -> Option<PasswdEntry> {
            if passwd.pw_name.is_null() {
                return None;
            }
            let login = unsafe { CStr::from_ptr(passwd.pw_name) }
                .to_string_lossy()
                .into_owned();
            let gecos = if passwd.pw_gecos.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(passwd.pw_gecos) }
                    .to_string_lossy()
                    .into_owned()
            };
            Some(PasswdEntry { login, gecos })
        }

        fn lookup_passwd_by_uid(uid: i64) -> Option<PasswdEntry> {
            let uid = libc::uid_t::try_from(uid).ok()?;
            let mut buffer_len = 1024usize;
            loop {
                let mut buffer = vec![0u8; buffer_len];
                let mut passwd = unsafe { std::mem::zeroed::<libc::passwd>() };
                let mut result = std::ptr::null_mut();
                let status = unsafe {
                    libc::getpwuid_r(
                        uid,
                        &mut passwd,
                        buffer.as_mut_ptr().cast(),
                        buffer.len(),
                        &mut result,
                    )
                };
                if status == 0 {
                    return (!result.is_null())
                        .then(|| passwd_entry_from_raw(&passwd))
                        .flatten();
                }
                if status != libc::ERANGE || buffer_len >= 1 << 20 {
                    return None;
                }
                buffer_len *= 2;
            }
        }

        fn lookup_passwd_by_login(login: &str) -> Option<PasswdEntry> {
            let login = CString::new(login).ok()?;
            let mut buffer_len = 1024usize;
            loop {
                let mut buffer = vec![0u8; buffer_len];
                let mut passwd = unsafe { std::mem::zeroed::<libc::passwd>() };
                let mut result = std::ptr::null_mut();
                let status = unsafe {
                    libc::getpwnam_r(
                        login.as_ptr(),
                        &mut passwd,
                        buffer.as_mut_ptr().cast(),
                        buffer.len(),
                        &mut result,
                    )
                };
                if status == 0 {
                    return (!result.is_null())
                        .then(|| passwd_entry_from_raw(&passwd))
                        .flatten();
                }
                if status != libc::ERANGE || buffer_len >= 1 << 20 {
                    return None;
                }
                buffer_len *= 2;
            }
        }

        pub(crate) fn effective_uid() -> i64 {
            unsafe { libc::geteuid() as i64 }
        }

        fn real_uid() -> i64 {
            unsafe { libc::getuid() as i64 }
        }

        fn capture_platform_identity() -> PlatformIdentity {
            let environment_login = login_name_from_env();
            PlatformIdentity {
                effective_passwd: lookup_passwd_by_uid(effective_uid()),
                environment_passwd: environment_login
                    .as_deref()
                    .and_then(lookup_passwd_by_login),
                environment_login,
                real_passwd: lookup_passwd_by_uid(real_uid()),
            }
        }
    }
    windows => {
        const SECONDARY_LOGIN_ENV: &str = "USERNAME";

        fn windows_passwd_entry() -> Option<PasswdEntry> {
            let login = whoami::fallible::username().ok()?;
            // GNU w32.c leaves its synthetic passwd `pw_gecos` empty.
            Some(PasswdEntry {
                login,
                gecos: String::new(),
            })
        }

        fn lookup_passwd_by_uid(uid: i64) -> Option<PasswdEntry> {
            (uid == effective_uid())
                .then(windows_passwd_entry)
                .flatten()
        }

        fn lookup_passwd_by_login(login: &str) -> Option<PasswdEntry> {
            let mut entry = windows_passwd_entry()?;
            let environment_alias = login_name_from_env();
            if !entry.login.eq_ignore_ascii_case(login)
                && environment_alias
                    .as_deref()
                    .is_none_or(|alias| !alias.eq_ignore_ascii_case(login))
            {
                return None;
            }
            entry.login = login.to_string();
            Some(entry)
        }

        pub(crate) fn effective_uid() -> i64 {
            windows_effective_uid().unwrap_or(123)
        }

        fn windows_effective_uid() -> Option<i64> {
            use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
            use windows_sys::Win32::Security::{
                GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TOKEN_QUERY,
                TOKEN_USER, TokenUser,
            };
            use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

            if whoami::fallible::username()
                .ok()
                .is_some_and(|name| name.eq_ignore_ascii_case("administrator"))
            {
                return Some(500);
            }

            let mut token: HANDLE = std::ptr::null_mut();
            if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
                return None;
            }

            let result = (|| {
                let mut required = 0u32;
                unsafe {
                    GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut required);
                }
                if required < std::mem::size_of::<TOKEN_USER>() as u32 {
                    return None;
                }
                let mut buffer = vec![0u8; required as usize];
                if unsafe {
                    GetTokenInformation(
                        token,
                        TokenUser,
                        buffer.as_mut_ptr().cast(),
                        required,
                        &mut required,
                    )
                } == 0
                {
                    return None;
                }

                let token_user = unsafe { &*(buffer.as_ptr().cast::<TOKEN_USER>()) };
                let sid = token_user.User.Sid;
                let count = unsafe { GetSidSubAuthorityCount(sid).as_ref().copied() }?;
                if count == 0 {
                    return None;
                }
                unsafe { GetSidSubAuthority(sid, u32::from(count - 1)).as_ref() }
                    .copied()
                    .map(i64::from)
            })();

            unsafe {
                CloseHandle(token);
            }
            result
        }

        fn capture_platform_identity() -> PlatformIdentity {
            // GNU w32.c synthesizes its passwd entry from the process token.
            // `whoami` uses the corresponding native Windows account APIs.
            let environment_login = login_name_from_env();
            let native_passwd = windows_passwd_entry();
            let environment_passwd = environment_login.as_ref().and_then(|login| {
                native_passwd.clone().map(|mut entry| {
                    entry.login = login.clone();
                    entry
                })
            });
            PlatformIdentity {
                effective_passwd: native_passwd.clone(),
                environment_login,
                environment_passwd,
                real_passwd: native_passwd,
            }
        }
    }
    _ => {
        const SECONDARY_LOGIN_ENV: &str = "USER";

        fn lookup_passwd_by_uid(_uid: i64) -> Option<PasswdEntry> {
            None
        }

        fn lookup_passwd_by_login(_login: &str) -> Option<PasswdEntry> {
            None
        }

        pub(crate) fn effective_uid() -> i64 {
            0
        }

        fn real_uid() -> i64 {
            effective_uid()
        }

        fn capture_platform_identity() -> PlatformIdentity {
            let environment_login = login_name_from_env();
            PlatformIdentity {
                effective_passwd: lookup_passwd_by_uid(effective_uid()),
                environment_passwd: environment_login
                    .as_deref()
                    .and_then(lookup_passwd_by_login),
                environment_login,
                real_passwd: lookup_passwd_by_uid(real_uid()),
            }
        }
    }
}

fn login_name_from_env() -> Option<String> {
    std::env::var_os("LOGNAME")
        .or_else(|| std::env::var_os(SECONDARY_LOGIN_ENV))
        .map(|name| name.to_string_lossy().into_owned())
}

pub(crate) fn lookup_login_by_uid(uid: i64) -> Option<String> {
    lookup_passwd_by_uid(uid).map(|entry| entry.login)
}

pub(crate) fn canonical_full_name(entry: &PasswdEntry) -> String {
    entry.gecos.split(',').next().unwrap_or("").to_string()
}

pub(crate) fn lookup_full_name_by_uid(uid: i64) -> Option<String> {
    lookup_passwd_by_uid(uid).map(|entry| canonical_full_name(&entry))
}

pub(crate) fn lookup_full_name_by_login(login: &str) -> Option<String> {
    lookup_passwd_by_login(login).map(|entry| canonical_full_name(&entry))
}

fn normalized_system_name() -> String {
    hostname::get()
        .map(|os| os.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "localhost".to_string())
        .chars()
        .map(|character| match character {
            ' ' | '\t' => '-',
            other => other,
        })
        .collect()
}

pub(crate) fn operating_system_release_value() -> Value {
    sysinfo::System::kernel_version()
        .map(Value::string)
        .unwrap_or(Value::NIL)
}

struct PlatformIdentity {
    effective_passwd: Option<PasswdEntry>,
    environment_login: Option<String>,
    environment_passwd: Option<PasswdEntry>,
    real_passwd: Option<PasswdEntry>,
}

impl PlatformIdentity {
    fn capture() -> Self {
        capture_platform_identity()
    }

    fn login_name(&self) -> String {
        self.environment_login
            .clone()
            .or_else(|| {
                self.effective_passwd
                    .as_ref()
                    .map(|entry| entry.login.clone())
            })
            .unwrap_or_else(|| "unknown".to_string())
    }

    fn real_login_name(&self) -> String {
        self.real_passwd
            .as_ref()
            .map(|entry| entry.login.clone())
            .unwrap_or_else(|| "unknown".to_string())
    }

    fn full_name(&self) -> String {
        if let Some(name) = std::env::var_os("NAME") {
            return name.to_string_lossy().into_owned();
        }

        let login = self.login_name();
        let entry = if login == self.real_login_name() {
            self.environment_passwd
                .as_ref()
                .or_else(|| {
                    self.effective_passwd
                        .as_ref()
                        .filter(|entry| entry.login == login)
                })
                .or_else(|| {
                    self.real_passwd
                        .as_ref()
                        .filter(|entry| entry.login == login)
                })
        } else {
            self.effective_passwd.as_ref()
        };
        entry
            .map(canonical_full_name)
            .unwrap_or_else(|| "unknown".to_string())
    }
}

pub(crate) struct RuntimeIdentity {
    operating_system_release: Value,
    system_name: String,
    user_full_name: Value,
    user_login_name: Value,
    user_real_login_name: Value,
}

impl RuntimeIdentity {
    pub(crate) fn capture() -> Self {
        let platform = PlatformIdentity::capture();
        Self {
            operating_system_release: operating_system_release_value(),
            system_name: normalized_system_name(),
            user_full_name: Value::string(platform.full_name()),
            user_login_name: Value::string(platform.login_name()),
            user_real_login_name: Value::string(platform.real_login_name()),
        }
    }

    pub(crate) fn install(self, eval: &mut Context) {
        install_system_name(eval, self.system_name);
        for (name, value) in [
            ("operating-system-release", self.operating_system_release),
            ("user-full-name", self.user_full_name),
            ("user-login-name", self.user_login_name),
            ("user-real-login-name", self.user_real_login_name),
        ] {
            eval.set_variable(name, value);
            eval.obarray_mut().make_special(name);
        }
    }
}

fn install_system_name(eval: &mut Context, name: String) {
    // GNU sysdep.c:init_system_name retains Vsystem_name when its bytes still
    // equal the current hostname.  This preserves object identity across
    // unchanged refreshes; only the refresh permission check below uses `eq`.
    let value = eval
        .obarray()
        .symbol_value("system-name")
        .copied()
        .filter(|value| value.as_utf8_str() == Some(name.as_str()))
        .unwrap_or_else(|| Value::string(name));
    eval.set_variable("system-name", value);
    eval.obarray_mut().make_special("system-name");
    eval.cached_system_name = value;
}

fn refresh_system_name_from(eval: &mut Context, name: String) {
    let visible = eval
        .obarray()
        .symbol_value("system-name")
        .copied()
        .unwrap_or(Value::NIL);
    if eq_value(&visible, &eval.cached_system_name) {
        install_system_name(eval, name);
    }
}

pub(crate) fn refresh_system_name(eval: &mut Context) {
    refresh_system_name_from(eval, normalized_system_name());
}

pub(crate) fn install(eval: &mut Context) {
    RuntimeIdentity::capture().install(eval);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_owned_system_name_refreshes_but_a_lisp_replacement_does_not() {
        let mut eval = Context::new();
        refresh_system_name_from(&mut eval, "host-after-refresh".to_string());
        assert_eq!(
            eval.obarray()
                .symbol_value("system-name")
                .and_then(|value| value.as_utf8_str()),
            Some("host-after-refresh")
        );

        // GNU compares object identity, not string contents: replacing the
        // variable with an equal, freshly allocated string is still a Lisp
        // override and must stop automatic refresh.
        eval.set_variable("system-name", Value::string("host-after-refresh"));
        refresh_system_name_from(&mut eval, "host-after-second-refresh".to_string());
        assert_eq!(
            eval.obarray()
                .symbol_value("system-name")
                .and_then(|value| value.as_utf8_str()),
            Some("host-after-refresh")
        );
    }

    std::cfg_select! {
        unix => {
            #[test]
            fn effective_uid_comes_from_the_os_without_a_child_process() {
                assert_eq!(effective_uid(), unsafe { libc::geteuid() as i64 });
            }
        }
        _ => {}
    }
}
