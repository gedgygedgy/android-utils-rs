use jni::{errors::Result, JNIEnv};

pub mod log;
pub mod os;

/// Initialize [`android-utils`](crate). This initializes the Android logger
/// implementation. This should be called before using
/// [`android-utils`](crate).
///
/// # Arguments
///
/// * `env` - Java environment to use.
pub fn init<'a: 'b, 'b>(env: &'b JNIEnv<'a>) -> Result<()> {
    log::init(env)?;
    Ok(())
}
