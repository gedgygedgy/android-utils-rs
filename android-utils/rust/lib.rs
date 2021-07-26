use jni::{errors::Result, JNIEnv};

pub mod os;
pub mod service;

/// Initialize [`android-utils`](crate). This currently does nothing, but it
/// may initialize some JNI functions in the future. This should be called
/// before using [`android-utils`](crate).
///
/// # Arguments
///
/// * `env` - Java environment to use.
pub fn init<'a: 'b, 'b>(_env: &'b JNIEnv<'a>) -> Result<()> {
    Ok(())
}
