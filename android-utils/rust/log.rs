use jni::{errors::Result, objects::JString, JNIEnv, JavaVM};
use log::{Level, LevelFilter, Log, Metadata, Record};
use once_cell::sync::OnceCell;

/// Calls `android.util.Log.d()`.
///
/// # Arguments
///
/// * `env` - Java environment to use.
/// * `tag` - Tag to use for logging.
/// * `msg` - Message to log.
pub fn d<'a: 'b, 'b>(env: &'b JNIEnv<'a>, tag: JString<'a>, msg: JString<'a>) -> Result<()> {
    env.call_static_method(
        "android/util/Log",
        "d",
        "(Ljava/lang/String;Ljava/lang/String;)I",
        &[tag.into(), msg.into()],
    )?;
    Ok(())
}

/// Calls `android.util.Log.e()`.
///
/// # Arguments
///
/// * `env` - Java environment to use.
/// * `tag` - Tag to use for logging.
/// * `msg` - Message to log.
pub fn e<'a: 'b, 'b>(env: &'b JNIEnv<'a>, tag: JString<'a>, msg: JString<'a>) -> Result<()> {
    env.call_static_method(
        "android/util/Log",
        "e",
        "(Ljava/lang/String;Ljava/lang/String;)I",
        &[tag.into(), msg.into()],
    )?;
    Ok(())
}

/// Calls `android.util.Log.i()`.
///
/// # Arguments
///
/// * `env` - Java environment to use.
/// * `tag` - Tag to use for logging.
/// * `msg` - Message to log.
pub fn i<'a: 'b, 'b>(env: &'b JNIEnv<'a>, tag: JString<'a>, msg: JString<'a>) -> Result<()> {
    env.call_static_method(
        "android/util/Log",
        "i",
        "(Ljava/lang/String;Ljava/lang/String;)I",
        &[tag.into(), msg.into()],
    )?;
    Ok(())
}

/// Calls `android.util.Log.v()`.
///
/// # Arguments
///
/// * `env` - Java environment to use.
/// * `tag` - Tag to use for logging.
/// * `msg` - Message to log.
pub fn v<'a: 'b, 'b>(env: &'b JNIEnv<'a>, tag: JString<'a>, msg: JString<'a>) -> Result<()> {
    env.call_static_method(
        "android/util/Log",
        "v",
        "(Ljava/lang/String;Ljava/lang/String;)I",
        &[tag.into(), msg.into()],
    )?;
    Ok(())
}

/// Calls `android.util.Log.w()`.
///
/// # Arguments
///
/// * `env` - Java environment to use.
/// * `tag` - Tag to use for logging.
/// * `msg` - Message to log.
pub fn w<'a: 'b, 'b>(env: &'b JNIEnv<'a>, tag: JString<'a>, msg: JString<'a>) -> Result<()> {
    env.call_static_method(
        "android/util/Log",
        "w",
        "(Ljava/lang/String;Ljava/lang/String;)I",
        &[tag.into(), msg.into()],
    )?;
    Ok(())
}

/// Calls `android.util.Log.wtf()`.
///
/// # Arguments
///
/// * `env` - Java environment to use.
/// * `tag` - Tag to use for logging.
/// * `msg` - Message to log.
pub fn wtf<'a: 'b, 'b>(env: &'b JNIEnv<'a>, tag: JString<'a>, msg: JString<'a>) -> Result<()> {
    env.call_static_method(
        "android/util/Log",
        "wtf",
        "(Ljava/lang/String;Ljava/lang/String;)I",
        &[tag.into(), msg.into()],
    )?;
    Ok(())
}

/// Calls `android.util.Log.println()`.
///
/// # Arguments
///
/// * `env` - Java environment to use.
/// * `priority` - Logging priority to use.
/// * `tag` - Tag to use for logging.
/// * `msg` - Message to log.
pub fn println<'a: 'b, 'b>(
    env: &'b JNIEnv<'a>,
    priority: i32,
    tag: JString<'a>,
    msg: JString<'a>,
) -> Result<()> {
    env.call_static_method(
        "android/util/Log",
        "println",
        "(ILjava/lang/String;Ljava/lang/String;)I",
        &[priority.into(), tag.into(), msg.into()],
    )?;
    Ok(())
}

/// Calls `android.util.Log.isLoggable()`.
///
/// # Arguments
///
/// * `env` - Java environment to use.
/// * `tag` - Tag to use for logging.
/// * `level` - Level to check.
pub fn is_loggable<'a: 'b, 'b>(env: &'b JNIEnv<'a>, tag: JString<'a>, level: i32) -> Result<bool> {
    env.call_static_method(
        "android/util/Log",
        "isLoggable",
        "(Ljava/lang/String;I)Z",
        &[tag.into(), level.into()],
    )?
    .z()
}

/// `android.util.Log.ASSERT`
pub const ASSERT: i32 = 7;

/// `android.util.Log.DEBUG`
pub const DEBUG: i32 = 3;

/// `android.util.Log.ERROR`
pub const ERROR: i32 = 6;

/// `android.util.Log.INFO`
pub const INFO: i32 = 4;

/// `android.util.Log.VERBOSE`
pub const VERBOSE: i32 = 2;

/// `android.util.Log.WARN`
pub const WARN: i32 = 5;

/// Convert a [`Level`] into a logging priority for `android.util.Log`.
pub fn log_level_to_priority(level: Level) -> i32 {
    match level {
        Level::Debug => DEBUG,
        Level::Error => ERROR,
        Level::Info => INFO,
        Level::Trace => VERBOSE,
        Level::Warn => WARN,
    }
}

struct AndroidLog(JavaVM);

struct DisableLogGuard(LevelFilter);

impl DisableLogGuard {
    pub fn new() -> Self {
        let old = log::max_level();
        log::set_max_level(LevelFilter::Off);
        Self(old)
    }
}

impl Drop for DisableLogGuard {
    fn drop(&mut self) {
        log::set_max_level(self.0);
    }
}

impl Log for AndroidLog {
    fn enabled(&self, metadata: &Metadata) -> bool {
        let _guard = DisableLogGuard::new();

        let env = self.0.get_env().unwrap();

        // Get any existing exceptions out of the way.
        let ex = if env.exception_check().unwrap() {
            let ex = env.exception_occurred().unwrap();
            env.exception_clear().unwrap();
            Some(ex)
        } else {
            None
        };

        let level = log_level_to_priority(metadata.level());
        let tag = env.new_string(metadata.target()).unwrap();
        let _tag_auto_local = env.auto_local(tag);
        let result = is_loggable(&env, tag, level).unwrap();

        // Restore the old exception.
        if let Some(ex) = ex {
            env.throw(ex).unwrap();
        }

        result
    }

    fn log(&self, record: &Record) {
        let _guard = DisableLogGuard::new();

        let env = self.0.get_env().unwrap();

        // Get any existing exceptions out of the way.
        let ex = if env.exception_check().unwrap() {
            let ex = env.exception_occurred().unwrap();
            env.exception_clear().unwrap();
            Some(ex)
        } else {
            None
        };

        let level = log_level_to_priority(record.level());
        let tag = env.new_string(record.target()).unwrap();
        let _tag_auto_local = env.auto_local(tag);

        if is_loggable(&env, tag, level).unwrap() {
            let msg = env.new_string(format!("{}", record.args())).unwrap();
            let _msg_auto_local = env.auto_local(msg);
            println(&env, level, tag, msg).unwrap();
        }

        // Restore the old exception.
        if let Some(ex) = ex {
            env.throw(ex).unwrap();
        }
    }

    fn flush(&self) {}
}

static ANDROID_LOG: OnceCell<AndroidLog> = OnceCell::new();

pub(crate) fn init<'a: 'b, 'b>(env: &'b JNIEnv<'a>) -> Result<()> {
    let vm = env.get_java_vm()?;
    let log = ANDROID_LOG.get_or_init(|| AndroidLog(vm));
    let _ = log::set_logger(log).map(|()| log::set_max_level(LevelFilter::max()));
    Ok(())
}
