use jni::{objects::JObject, JNIEnv};
use jni_utils::exceptions::throw_unwind;

#[no_mangle]
pub extern "C" fn Java_io_github_gedgygedgy_rust_android_AndroidTest_testPanicInternal(
    env: JNIEnv,
    _obj: JObject,
) {
    let _ = throw_unwind(&env, || {
        panic!("testPanic() panicked");
    });
}
