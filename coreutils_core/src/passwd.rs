//! A module to deal more easily with UNIX passwd.

use std::{
    error::Error as StdError,
    ffi::CStr,
    fmt::{self, Display},
    mem, ptr,
};

use crate::group::{Error as GrError, Gid, Groups};

use self::Error::*;

use libc::{geteuid, getpwnam_r, getpwuid_r, uid_t};

use bstr::{BStr, BString};

/// User ID type.
pub type Uid = uid_t;

pub type Result<T> = std::result::Result<T, Error>;

/// This struct holds information about a passwd of UNIX/UNIX-like systems.
///
/// Contains `sys/types.h` `passwd` struct attributes as Rust more powefull types.
#[derive(Debug)]
pub enum Error {
    /// Happens when `getpwgid_r` or `getpwnam_r` fails.
    ///
    /// It holds the the function that was used and a error code of the function return.
    GetPasswdFailed(String, i32),
    /// Happens when the pointer to the `.pw_name` is NULL.
    NameCheckFailed,
    /// Happens when the pointer to the `.pw_passwd` is NULL.
    PasswdCheckFailed,
    /// Happens when the pointer to the `.pw_gecos` is NULL.
    GecosCheckFailed,
    /// Happens when the pointer to the `.pw_dir` is NULL.
    DirCheckFailed,
    /// Happens when the pointer to the `.pw_shell` is NULL.
    ShellCheckFailed,
    /// Happens when the passwd is not found.
    PasswdNotFound,
    /// Happens when something happens when finding what `Group` a `Passwd` belongs
    Group(Box<GrError>),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GetPasswdFailed(fn_name, err_code) => write!(
                f,
                "Failed to get group with the following error code: {}. For more info search for the {} manual",
                err_code, fn_name
            ),
            NameCheckFailed => write!(f, "Group name check failed, `.pw_name` field is null"),
            PasswdCheckFailed => write!(f, "Group passwd check failed, `.pw_passwd` is null"),
            GecosCheckFailed => write!(f, "Group gecos check failed, `.pw_gecos` is null"),
            DirCheckFailed => write!(f, "Group dir check failed, `.pw_dir` is null"),
            ShellCheckFailed => write!(f, "Group shell check failed, `.pw_shell` is null"),
            PasswdNotFound => write!(f, "Passwd was not found in the system"),
            Group(err) => write!(f, "The following error hapenned trying to get all `Groups`: {}", err),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        None
    }
}

impl From<GrError> for Error {
    fn from(err: GrError) -> Error {
        Group(Box::new(err))
    }
}

/// This struct holds the information of a user in UNIX/UNIX-like systems.
///
/// Contains `sys/types.h` `passwd` struct attributes as Rust more common types.
// It also contains a pointer to the libc::passwd type for more complex manipulations.
#[derive(Clone, Debug)]
pub struct Passwd {
    /// User login name.
    name: BString,
    /// User encrypted password.
    passwd: BString,
    /// User ID.
    user_id: Uid,
    /// User Group ID.
    group_id: Gid,
    /// User full name.
    gecos: BString,
    /// User directory.
    dir: BString,
    /// User login shell
    shell: BString,
    // pw: *mut passwd,
}

impl Passwd {
    /// Create a new `Passwd` getting the user passwd as default.
    ///
    /// It may fail, so return a `Result`, either the `Passwd` struct wrapped in a `Ok`, or
    /// a `Error` wrapped in a `Err`.
    pub fn new() -> Result<Self> {
        let mut buff = [0; 16384]; // Got this size from manual page about getpwuid_r
        let mut pw: libc::passwd = unsafe { mem::zeroed() };
        let mut pw_ptr = ptr::null_mut();

        let res = unsafe { getpwuid_r(geteuid(), &mut pw, &mut buff[0], buff.len(), &mut pw_ptr) };

        if pw_ptr.is_null() {
            if res == 0 {
                return Err(PasswdNotFound);
            } else {
                return Err(GetPasswdFailed(String::from("getpwnam_r"), res));
            }
        }

        let name = if !pw.pw_name.is_null() {
            let name_cstr = unsafe { CStr::from_ptr(pw.pw_name) };
            BString::from_slice(name_cstr.to_bytes())
        } else {
            return Err(NameCheckFailed);
        };

        let passwd = if !pw.pw_passwd.is_null() {
            let passwd_cstr = unsafe { CStr::from_ptr(pw.pw_passwd) };
            BString::from_slice(passwd_cstr.to_bytes())
        } else {
            return Err(PasswdCheckFailed);
        };

        let user_id = pw.pw_uid;

        let group_id = pw.pw_gid;

        let gecos = if !pw.pw_gecos.is_null() {
            let gecos_cstr = unsafe { CStr::from_ptr(pw.pw_gecos) };
            BString::from_slice(gecos_cstr.to_bytes())
        } else {
            return Err(GecosCheckFailed);
        };

        let dir = if !pw.pw_dir.is_null() {
            let dir_cstr = unsafe { CStr::from_ptr(pw.pw_dir) };
            BString::from_slice(dir_cstr.to_bytes())
        } else {
            return Err(DirCheckFailed);
        };

        let shell = if !pw.pw_shell.is_null() {
            let shell_cstr = unsafe { CStr::from_ptr(pw.pw_shell) };
            BString::from_slice(shell_cstr.to_bytes())
        } else {
            return Err(ShellCheckFailed);
        };

        Ok(Passwd {
            name,
            passwd,
            user_id,
            group_id,
            gecos,
            dir,
            shell,
            // pw: &mut pw,
        })
    }

    /// Create a new `Passwd` using a `id` to get all attributes.
    ///
    /// It may fail, so return a `Result`, either the `Passwd` struct wrapped in a `Ok`, or
    /// a `Error` wrapped in a `Err`.
    pub fn from_uid(id: Uid) -> Result<Self> {
        let mut buff = [0; 16384]; // Got this size from manual page about getpwuid_r
        let mut pw: libc::passwd = unsafe { mem::zeroed() };
        let mut pw_ptr = ptr::null_mut();

        let res = unsafe { getpwuid_r(id, &mut pw, &mut buff[0], buff.len(), &mut pw_ptr) };

        if pw_ptr.is_null() {
            if res == 0 {
                return Err(PasswdNotFound);
            } else {
                return Err(GetPasswdFailed(String::from("getpwnam_r"), res));
            }
        }

        let name_ptr = pw.pw_name;
        let passwd_ptr = pw.pw_passwd;
        let gecos_ptr = pw.pw_gecos;
        let dir_ptr = pw.pw_dir;
        let shell_ptr = pw.pw_shell;

        let name = if !name_ptr.is_null() {
            let name_cstr = unsafe { CStr::from_ptr(name_ptr) };
            BString::from_slice(name_cstr.to_bytes())
        } else {
            return Err(NameCheckFailed);
        };

        let passwd = if !passwd_ptr.is_null() {
            let passwd_cstr = unsafe { CStr::from_ptr(passwd_ptr) };
            BString::from_slice(passwd_cstr.to_bytes())
        } else {
            return Err(PasswdCheckFailed);
        };

        let user_id = id;

        let group_id = pw.pw_gid;

        let gecos = if !gecos_ptr.is_null() {
            let gecos_cstr = unsafe { CStr::from_ptr(gecos_ptr) };
            BString::from_slice(gecos_cstr.to_bytes())
        } else {
            return Err(GecosCheckFailed);
        };

        let dir = if !dir_ptr.is_null() {
            let dir_cstr = unsafe { CStr::from_ptr(dir_ptr) };
            BString::from_slice(dir_cstr.to_bytes())
        } else {
            return Err(DirCheckFailed);
        };

        let shell = if !shell_ptr.is_null() {
            let shell_cstr = unsafe { CStr::from_ptr(shell_ptr) };
            BString::from_slice(shell_cstr.to_bytes())
        } else {
            return Err(ShellCheckFailed);
        };

        Ok(Passwd {
            name,
            passwd,
            user_id,
            group_id,
            gecos,
            dir,
            shell,
            // pw,
        })
    }

    /// Create a new `Passwd` using a `name` to get all attributes.
    ///
    /// It may fail, so return a `Result`, either the `Passwd` struct wrapped in a `Ok`, or
    /// a `Error` wrapped in a `Err`.
    pub fn from_name(name: &str) -> Result<Self> {
        let mut pw: libc::passwd = unsafe { mem::zeroed() };
        let mut pw_ptr = ptr::null_mut();
        let mut buff = [0; 16384]; // Got this size from manual page about getpwuid_r

        let name = BString::from_slice(name);

        let res = unsafe {
            getpwnam_r(
                name.as_ptr() as *const i8,
                &mut pw,
                &mut buff[0],
                buff.len(),
                &mut pw_ptr,
            )
        };

        if pw_ptr.is_null() {
            if res == 0 {
                return Err(PasswdNotFound);
            } else {
                return Err(GetPasswdFailed(String::from("getpwnam_r"), res));
            }
        }

        let passwd_ptr = pw.pw_passwd;
        let gecos_ptr = pw.pw_gecos;
        let dir_ptr = pw.pw_dir;
        let shell_ptr = pw.pw_shell;

        let passwd = if !passwd_ptr.is_null() {
            let passwd_cstr = unsafe { CStr::from_ptr(passwd_ptr) };
            BString::from_slice(passwd_cstr.to_bytes())
        } else {
            return Err(PasswdCheckFailed);
        };

        let user_id = pw.pw_uid;

        let group_id = pw.pw_gid;

        let gecos = if !gecos_ptr.is_null() {
            let gecos_cstr = unsafe { CStr::from_ptr(gecos_ptr) };
            BString::from_slice(gecos_cstr.to_bytes())
        } else {
            return Err(GecosCheckFailed);
        };

        let dir = if !dir_ptr.is_null() {
            let dir_cstr = unsafe { CStr::from_ptr(dir_ptr) };
            BString::from_slice(dir_cstr.to_bytes())
        } else {
            return Err(DirCheckFailed);
        };

        let shell = if !shell_ptr.is_null() {
            let shell_cstr = unsafe { CStr::from_ptr(shell_ptr) };
            BString::from_slice(shell_cstr.to_bytes())
        } else {
            return Err(ShellCheckFailed);
        };

        Ok(Passwd {
            name,
            passwd,
            user_id,
            group_id,
            gecos,
            dir,
            shell,
            // pw,
        })
    }

    /// Get `Passwd` login name.
    pub fn name(&self) -> &BStr {
        &self.name
    }

    /// Get `Passwd` encrypted password.
    pub fn passwd(&self) -> &BStr {
        &self.passwd
    }

    /// Get `Passwd` user ID.
    pub fn uid(&self) -> Uid {
        self.user_id
    }

    /// Get `Passwd` group ID.
    pub fn gid(&self) -> Gid {
        self.group_id
    }

    /// Get `Passwd` full name.
    pub fn gecos(&self) -> &BStr {
        &self.gecos
    }

    /// Get `Passwd` dir.
    pub fn dir(&self) -> &BStr {
        &self.dir
    }

    /// Get `Passwd` shell.
    pub fn shell(&self) -> &BStr {
        &self.shell
    }

    /// Get the groups that `Passwd` belongs to.
    pub fn belongs_to(&self) -> Result<Groups> {
        let gr = Groups::from_passwd(self)?;
        Ok(gr)
    }

    // /// Get the raw pointer to the passwd.
    // pub fn raw_ptr(&self) -> *const passwd {
    //     self.pw
    // }
    //
    // // Get a mutable raw pointer to the passwd.
    // // Use with caution.
    // pub unsafe fn raw_ptr_mut(&mut self) -> *mut passwd {
    //     self.pw
    // }
}
